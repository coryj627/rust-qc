use git2::{Config, Error, ObjectType, Oid, Repository};
use std::io::{Read, Write};

#[test]
fn oid_boundary_inputs() {
    for s in [
        "",
        "xyz",
        "00\0",
        "é",
        "00000000000000000000000000000000000000000",
    ] {
        assert!(Oid::from_str(s).is_err());
    }
    for n in 1..=40 {
        let text = "1".repeat(n);
        let oid = Oid::from_str(&text).unwrap();
        assert_eq!(oid.to_string(), format!("{text:0<40}"));
    }
    assert!(Oid::from_bytes(&[0; 19]).is_err());
    assert!(Oid::from_bytes(&[0; 21]).is_err());
    assert_eq!(Oid::from_bytes(&[0; 20]).unwrap(), Oid::zero());
}

#[test]
fn typed_config_boundaries_and_duplicate_values() -> Result<(), Error> {
    assert_eq!(Config::parse_bool("yes")?, true);
    assert_eq!(Config::parse_bool("")?, false);
    assert_eq!(Config::parse_i32("2k")?, 2048);
    assert!(Config::parse_i32("2147483648").is_err());
    assert!(Config::parse_i64("9223372036854775808").is_err());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config");
    std::fs::write(&path, "[test]\nvalue = first\nvalue = second\nflag\n").unwrap();
    let mut config = Config::open(&path)?;
    assert_eq!(config.get_string("test.value")?, "second");
    assert!(config.get_bool("test.flag")?);
    let values = config.multivar("test.value", None)?;
    let mut all = Vec::new();
    values.for_each(|entry| all.push(entry.value().unwrap().to_owned()))?;
    assert_eq!(all, ["first", "second"]);
    let snapshot = config.snapshot()?;
    assert_eq!(snapshot.get_str("test.flag")?, "");
    Ok(())
}

#[test]
fn unicode_and_duplicate_message_trailers() -> Result<(), Error> {
    let input = "Résumé\n\nReviewed-by: Zoë\nReviewed-by: 李\n";
    let trailers = git2::message_trailers_strs(input)?;
    assert_eq!(trailers.len(), 2);
    assert_eq!(
        trailers.iter().collect::<Vec<_>>(),
        [("Reviewed-by", "Zoë"), ("Reviewed-by", "李")]
    );
    assert_eq!(git2::message_prettify("Résumé\n\n\n", None)?, "Résumé\n");
    Ok(())
}

#[test]
fn loose_object_stream_supports_short_reads_and_eof() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let repo = Repository::init(dir.path())?;
    let odb = repo.odb()?;
    let mut writer = odb.writer(5, ObjectType::Blob)?;
    writer.write_all(b"hello")?;
    let id = writer.finalize()?;
    assert_eq!(odb.read(id)?.data(), b"hello");
    let (mut reader, len, kind) = odb.reader(id)?;
    assert_eq!((len, kind), (5, ObjectType::Blob));
    let mut result = Vec::new();
    let mut chunk = [0; 2];
    loop {
        let n = reader.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        result.extend_from_slice(&chunk[..n]);
    }
    assert_eq!(result, b"hello");
    Ok(())
}

#[test]
fn helper_receives_domain_host() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config");
    std::fs::write(&path, "[credential]\n\thelper = \"!f() { while IFS= read -r line; do case $line in host=*) echo username=${line#host=};; esac; done; echo password=dummy; }; f\"\n").unwrap();
    let config = Config::open(&path).unwrap();
    assert_eq!(
        git2::CredentialHelper::new("https://example.com/project")
            .config(&config)
            .execute(),
        Some(("example.com".into(), "dummy".into()))
    );
}
