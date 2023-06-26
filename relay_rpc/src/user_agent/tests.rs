use super::{
    Environment,
    Id,
    OsInfo,
    ParsingError,
    Protocol,
    ProtocolKind,
    Sdk,
    SdkLanguage,
    UserAgent,
    ValidUserAgent,
};

#[test]
fn parse_protocol() {
    let str_good = "wc-2";
    let str_good_unknown = "unknown-2";
    let str_bad = "bad";

    let good = str_good.parse::<Protocol>().unwrap();
    let good_unknown = str_good_unknown.parse::<Protocol>().unwrap();
    let bad = str_bad.parse::<Protocol>();

    assert_eq!(good, Protocol {
        kind: ProtocolKind::WalletConnect,
        version: 2
    });

    assert_eq!(good_unknown, Protocol {
        kind: ProtocolKind::Unknown("unknown".to_owned()),
        version: 2
    });

    assert_eq!(bad.unwrap_err(), ParsingError::Protocol);

    assert_eq!(str_good, &good.to_string());

    assert_eq!(str_good_unknown, &good_unknown.to_string());
}

#[test]
fn parse_sdk() {
    let str_good = "swift-2.0.0-rc.1";
    let str_good_unknown = "unknown-2.0.0-rc.1";
    let str_bad = "bad";

    let good = str_good.parse::<Sdk>().unwrap();
    let good_unknown = str_good_unknown.parse::<Sdk>().unwrap();
    let bad = str_bad.parse::<Sdk>();

    assert_eq!(good, Sdk {
        language: SdkLanguage::Swift,
        version: "2.0.0-rc.1".to_owned()
    });

    assert_eq!(good_unknown, Sdk {
        language: SdkLanguage::Unknown("unknown".to_owned()),
        version: "2.0.0-rc.1".to_owned()
    });

    assert_eq!(bad.unwrap_err(), ParsingError::Sdk);

    assert_eq!(str_good, &good.to_string());

    assert_eq!(str_good_unknown, &good_unknown.to_string());
}

#[test]
fn parse_id() {
    let str_good = "browser:app.example.org";
    let str_good_unknown = "unknown:app.example.org";
    let str_bad = "";

    let good = str_good.parse::<Id>().unwrap();
    let good_unknown = str_good_unknown.parse::<Id>().unwrap();
    let bad = str_bad.parse::<Id>();

    assert_eq!(good, Id {
        environment: Environment::Browser,
        host: Some("app.example.org".to_owned())
    });

    assert_eq!(good_unknown, Id {
        environment: Environment::Unknown("unknown".to_owned()),
        host: Some("app.example.org".to_owned())
    });

    assert_eq!(bad.unwrap_err(), ParsingError::Id);

    assert_eq!(str_good, &good.to_string());

    assert_eq!(str_good_unknown, &good_unknown.to_string());
}

#[test]
fn parse_valid_ua() {
    let str_good = "wc-2/js-2.0.0-rc.1/ios-12.4";
    let str_good_with_id = "wc-2/js-2.0.0-rc.1/ios-12.4/browser:app.example.org";
    let str_bad = "bad";

    let good = str_good.parse::<ValidUserAgent>().unwrap();
    let good_with_id = str_good_with_id.parse::<ValidUserAgent>().unwrap();
    let bad = str_bad.parse::<ValidUserAgent>();

    assert_eq!(good, ValidUserAgent {
        protocol: Protocol {
            kind: ProtocolKind::WalletConnect,
            version: 2
        },
        sdk: Sdk {
            language: SdkLanguage::Js,
            version: "2.0.0-rc.1".to_owned()
        },
        os: OsInfo {
            os_family: "ios".to_owned(),
            ua_family: None,
            version: Some("12.4".to_owned())
        },
        id: None
    });

    assert_eq!(good_with_id, ValidUserAgent {
        protocol: Protocol {
            kind: ProtocolKind::WalletConnect,
            version: 2
        },
        sdk: Sdk {
            language: SdkLanguage::Js,
            version: "2.0.0-rc.1".to_owned()
        },
        os: OsInfo {
            os_family: "ios".to_owned(),
            ua_family: None,
            version: Some("12.4".to_owned())
        },
        id: Some(Id {
            environment: Environment::Browser,
            host: Some("app.example.org".to_owned())
        })
    });

    assert_eq!(bad.unwrap_err(), ParsingError::UserAgent);

    assert_eq!(str_good, &good.to_string());

    assert_eq!(str_good_with_id, &good_with_id.to_string());
}

#[test]
fn parse_ua() {
    let str_good = "wc-2/js-2.0.0-rc.1/ios-12.4";
    let str_good_unknown = "unknown";
    let str_bad = "";

    let good = str_good.parse::<UserAgent>().unwrap();
    let good_unknown = str_good_unknown.parse::<UserAgent>().unwrap();
    let bad = str_bad.parse::<UserAgent>();

    assert_eq!(
        good,
        UserAgent::ValidUserAgent(ValidUserAgent {
            protocol: Protocol {
                kind: ProtocolKind::WalletConnect,
                version: 2
            },
            sdk: Sdk {
                language: SdkLanguage::Js,
                version: "2.0.0-rc.1".to_owned()
            },
            os: OsInfo {
                os_family: "ios".to_owned(),
                ua_family: None,
                version: Some("12.4".to_owned())
            },
            id: None
        })
    );

    assert_eq!(
        good_unknown,
        UserAgent::Unknown(str_good_unknown.to_owned())
    );

    assert_eq!(bad.unwrap_err(), ParsingError::UserAgent);

    assert_eq!(str_good, &good.to_string());

    assert_eq!(str_good_unknown, &good_unknown.to_string());
}

#[test]
fn parse_os() {
    let fixtures = vec![
        ("windowsxp-ie-7.0.1", OsInfo {
            os_family: "windowsxp".to_owned(),
            ua_family: Some("ie".to_owned()),
            version: Some("7.0.1".to_owned()),
        }),
        ("windows7-edge-chromium-90.0.818", OsInfo {
            os_family: "windows7".to_owned(),
            ua_family: Some("edge-chromium".to_owned()),
            version: Some("90.0.818".to_owned()),
        }),
        ("win32-18.7.0", OsInfo {
            os_family: "win32".to_owned(),
            ua_family: None,
            version: Some("18.7.0".to_owned()),
        }),
        ("iPadOS-16.1", OsInfo {
            os_family: "ipados".to_owned(),
            ua_family: None,
            version: Some("16.1".to_owned()),
        }),
        ("android-9", OsInfo {
            os_family: "android".to_owned(),
            ua_family: None,
            version: Some("9".to_owned()),
        }),
    ];

    for (os_str, info) in &fixtures {
        let parsed_info: OsInfo = os_str.parse().unwrap();
        assert_eq!(info, &parsed_info);
        assert_eq!(os_str.to_lowercase(), parsed_info.to_string());
    }

    assert_eq!("".parse::<OsInfo>(), Err(ParsingError::Os));
}
