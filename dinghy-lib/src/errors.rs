error_chain! {
    foreign_links {
        Io(::std::io::Error);
        StringFromUtf8(::std::string::FromUtf8Error);
        PathStripPrefix(::std::path::StripPrefixError);
        CargoMetadata(::cargo_metadata::Error);
        Plist(::plist::Error);
        Regex(::regex::Error);
        Json(::json::Error);
        Ignore(::ignore::Error);
        Toml(::toml::de::Error);
        SerdeJson(::serde_json::Error);
    }

    links {
        Rexpect(::rexpect::errors::Error, ::rexpect::errors::ErrorKind);
    }

    errors {
        Child(code: i32) {
        }
        PackagesCannotBeCompiledForPlatform(packages: Vec<String>) {
            description("Cannot compile selected packages for the selected platform")
            display("{:?} cannot be compiled for the selected platform (see project's [package.metadata.dinghy] in Cargo.toml)", packages)
        }
    }
}

impl From<::std::process::ExitStatus> for Error {
    fn from(ex: ::std::process::ExitStatus) -> Error {
        assert!(!ex.success());
        match ex.code() {
            Some(i) => ::errors::ErrorKind::Child(i).into(),
            None => "No error code, child killed by some signal ?".into()
        }
    }
}
