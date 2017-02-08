error_chain!{
    foreign_links {
        ::std::io::Error, IoError;
        ::std::string::FromUtf8Error, StringFromUtf8Error;
        ::std::path::StripPrefixError, PathStripPrefixError;
        Box<::cargo::CargoError>, CargoError;
        ::plist::Error, PlistError;
        ::regex::Error, RegexError;
        ::json::Error, JsonError;
        ::ignore::Error, IgnoreError;
        ::toml::DecodeError, TomlDecodeError;
    }
}
