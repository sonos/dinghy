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
        PackagesCannotBeCompiledForPlatform(packages: Vec<String>) {
            description("Cannot compile selected packages for the selected platform")
            display("{:?} cannot be compiled for the selected platform (see project's [package.metadata.dinghy] in Cargo.toml)", packages)
        }
    }
}

