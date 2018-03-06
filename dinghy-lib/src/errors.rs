error_chain! {
    foreign_links {
        Io(::std::io::Error);
        StringFromUtf8(::std::string::FromUtf8Error);
        PathStripPrefix(::std::path::StripPrefixError);
        Cargo(::cargo::CargoError);
        Plist(::plist::Error);
        Regex(::regex::Error);
        Json(::json::Error);
        Ignore(::ignore::Error);
        Toml(::toml::de::Error);
    }
}
