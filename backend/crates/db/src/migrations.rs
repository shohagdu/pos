//! The embedded migrations, runnable on MySQL 8 and MariaDB 10.6+.

use std::{borrow::Cow, sync::LazyLock};

use sqlx::migrate::Migrator;

static EMBEDDED: Migrator = sqlx::migrate!("../../migrations");

/// Migrations edited after release to run on MariaDB too (`UUID_TO_BIN`,
/// `REGEXP_LIKE` and the 0900 collation are MySQL-only). The SQL has the same
/// effect, so databases that already applied them keep their recorded
/// SHA-384 checksum instead of failing as "previously applied but modified".
const RELEASED_CHECKSUMS: &[(i64, &str)] = &[
    (1, "b768fae73f551fd099b92857e0e5324b6ff09c784af0f916bd3623e0855461fcc34cf994d2898800f98580f5a356dd74"),
    (2, "1dbe243dfdcc68df87c2ad4bad04383c8ba6d615abfaf0527999e6a15678e44f96557869d5b7d0dfd8be198f122226ca"),
    (3, "a7a2ade52fc6c5274acf8badded3ea6bf5e3b890e8e52d3154f4f6715883d3ddd3164045ef1981e00702a330b6e60dc8"),
    (4, "167129c908977538d3099efc4e3aa6d475d91ec57af1fc84fd5c5055918c90e9c8701830e0e93f45635a81d4f30ea41a"),
    (9, "d0144288ef61e76f5b06e043c4c6c228c3db838d5fefaf71427ba333e4b8dc555bc56af4d79cbc986e6afa1e612c8bf9"),
    (12, "ea2b28c69dd366d85ac57eaa9db65362c46f058ff44f24d2d799c8bd9598b389c89fde79e003d80049b0817cb4407612"),
    (13, "2c458a45ce182af3269719314a9ba9171c989da1b5725a49069b5781d4b46690a6a0d4035fbbe497a8c4c1f6f3a0ea3c"),
    (17, "2a1f8a89c3bdd79462a19687b3172169d79b5a84716daacfad1333d2ba0f4f372dc9586a0860f6590fe298206e598202"),
    (101, "6ffd5c3a811bf316ac90135549bcc198cdc07671f62c69b741790ea15c1b38416a4117b92244e5bcc5e2f53be07992f2"),
    (103, "1eb8cfce0ce67cfc073551e40f3645c09be4b3707d44834ef1ac7621fed33fc91ef0b4bec9a6066482bb924492bec5d7"),
    (105, "4103f34bbe4e9bdcba107317b2e36ad900f7e1d6b15eb345f86326cf7894087a908a59b58c531263d6c84626c9966823"),
    (106, "080d289fca345f593ff2f7f3f6d61c45dde8dad5edd67dd9e108f839f15d9c7d660443fbfb79b728964d5b680b0b21af"),
];

/// Embedded migrations from `backend/migrations` (compiled into the binary).
pub static MIGRATOR: LazyLock<Migrator> = LazyLock::new(|| {
    let migrations = EMBEDDED
        .migrations
        .iter()
        .map(|m| {
            let mut m = m.clone();
            if let Some((_, hex)) = RELEASED_CHECKSUMS.iter().find(|(v, _)| *v == m.version) {
                m.checksum = Cow::Owned(decode_hex(hex));
            }
            m
        })
        .collect::<Vec<_>>();
    Migrator {
        migrations: Cow::Owned(migrations),
        ignore_missing: EMBEDDED.ignore_missing,
        locking: EMBEDDED.locking,
        no_tx: EMBEDDED.no_tx,
        table_name: EMBEDDED.table_name.clone(),
        create_schemas: EMBEDDED.create_schemas.clone(),
    }
});

fn decode_hex(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("RELEASED_CHECKSUMS holds hex"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn released_checksums_are_sha384() {
        for (version, hex) in RELEASED_CHECKSUMS {
            assert_eq!(decode_hex(hex).len(), 48, "migration {version}");
            assert!(EMBEDDED.migrations.iter().any(|m| m.version == *version), "migration {version} not embedded");
        }
        assert_eq!(MIGRATOR.migrations.len(), EMBEDDED.migrations.len());
    }
}
