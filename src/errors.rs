error_chain! {
    foreign_links {
        IoError(::std::io::Error);
        StringFromUtf8Error(::std::string::FromUtf8Error);
        PathStripPrefixError(::std::path::StripPrefixError);
        CargoError(Box<::cargo::CargoError>);
        PlistError(::plist::Error);
        RegexError(::regex::Error);
        JsonError(::json::Error);
        IgnoreError(::ignore::Error);
        TomlDecodeError(::toml::de::Error);
    }
}
