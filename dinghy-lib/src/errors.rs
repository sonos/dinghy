use failure;
error_chain! {
    foreign_links {
        Io(::std::io::Error);
        StringFromUtf8(::std::string::FromUtf8Error);
        PathStripPrefix(::std::path::StripPrefixError);
        Plist(::plist::Error);
        Regex(::regex::Error);
        Json(::json::Error);
        Ignore(::ignore::Error);
        Toml(::toml::de::Error);
    }

    errors {
        PackagesCannotBeCompiledForPlatform(packages: Vec<String>) {
            description("Cannot compile selected packages for the selected platform")
            display("{:?} cannot be compiled for the selected platform (see project's [package.metadata.dinghy] in Cargo.toml)", packages)
        }
        Cargo(err: String) {
            description("A cargo error")
            display("{:?}", err)
        }
    }
}
impl From<failure::Error> for Error {
    fn from(err: failure::Error) -> Error {
        Error::from_kind(ErrorKind::Cargo(err.to_string()))
    }
}
