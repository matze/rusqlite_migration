use std::{iter::FromIterator, num::NonZeroUsize};

use rusqlite::{Connection, Transaction};

use crate::{
    user_version, Error, ForeignKeyCheckError, MigrationDefinitionError, Migrations, SchemaVersion,
    SchemaVersionError, M,
};

fn m_valid0() -> M<'static> {
    M::up("CREATE TABLE m1(a, b); CREATE TABLE m2(a, b, c);")
}
fn m_valid10() -> M<'static> {
    M::up("CREATE TABLE t1(a, b);")
}
fn m_valid11() -> M<'static> {
    M::up("ALTER TABLE t1 RENAME COLUMN b TO c;")
}
fn m_valid20() -> M<'static> {
    M::up("CREATE TABLE t2(b);")
}
fn m_valid21() -> M<'static> {
    M::up("ALTER TABLE t2 ADD COLUMN a;")
}

fn m_valid_fk() -> M<'static> {
    M::up(
        "CREATE TABLE fk1(a PRIMARY KEY); \
        CREATE TABLE fk2( \
            a, \
            FOREIGN KEY(a) REFERENCES fk1(a) \
        ); \
        INSERT INTO fk1 (a) VALUES ('foo'); \
        INSERT INTO fk2 (a) VALUES ('foo'); \
    ",
    )
    .foreign_key_check()
}

// All valid Ms in the right order
fn all_valid() -> Vec<M<'static>> {
    vec![
        m_valid0(),
        m_valid10(),
        m_valid11(),
        m_valid20(),
        m_valid21(),
        m_valid_fk(),
    ]
}

fn m_invalid0() -> M<'static> {
    M::up("CREATE TABLE table3()")
}
fn m_invalid1() -> M<'static> {
    M::up("something invalid")
}

fn m_invalid_fk() -> M<'static> {
    M::up(
        "CREATE TABLE fk1(a PRIMARY KEY); \
        CREATE TABLE fk2( \
            a, \
            FOREIGN KEY(a) REFERENCES fk1(a) \
        ); \
        INSERT INTO fk2 (a) VALUES ('foo'); \
    ",
    )
    .foreign_key_check()
}

#[test]
fn empty_migrations_test() {
    let mut conn = Connection::open_in_memory().unwrap();
    let m = Migrations::new(vec![]);

    assert_eq!(
        Err(Error::MigrationDefinition(
            MigrationDefinitionError::NoMigrationsDefined
        )),
        m.to_latest(&mut conn)
    );

    for v in 0..4 {
        assert_eq!(
            Err(Error::MigrationDefinition(
                MigrationDefinitionError::NoMigrationsDefined
            )),
            m.to_version(&mut conn, v)
        )
    }
}

#[test]
fn schema_version_partial_cmp_test() {
    assert_eq!(SchemaVersion::NoneSet, SchemaVersion::NoneSet);
    assert_eq!(
        SchemaVersion::Inside(NonZeroUsize::new(1).unwrap()),
        SchemaVersion::Inside(NonZeroUsize::new(1).unwrap())
    );
    assert_eq!(
        SchemaVersion::Outside(NonZeroUsize::new(1).unwrap()),
        SchemaVersion::Outside(NonZeroUsize::new(1).unwrap())
    );
    assert_ne!(
        SchemaVersion::Outside(NonZeroUsize::new(1).unwrap()),
        SchemaVersion::Inside(NonZeroUsize::new(1).unwrap())
    );
    assert_ne!(
        SchemaVersion::Outside(NonZeroUsize::new(1).unwrap()),
        SchemaVersion::NoneSet
    );
    assert_ne!(
        SchemaVersion::Inside(NonZeroUsize::new(1).unwrap()),
        SchemaVersion::NoneSet
    );
    assert!(SchemaVersion::NoneSet < SchemaVersion::Inside(NonZeroUsize::new(1).unwrap()));
    assert!(SchemaVersion::NoneSet < SchemaVersion::Outside(NonZeroUsize::new(1).unwrap()));
    assert!(
        SchemaVersion::Inside(NonZeroUsize::new(1).unwrap())
            < SchemaVersion::Outside(NonZeroUsize::new(2).unwrap())
    );
    assert!(
        SchemaVersion::Outside(NonZeroUsize::new(1).unwrap())
            < SchemaVersion::Inside(NonZeroUsize::new(2).unwrap())
    );
}

#[test]
fn test_migration_hook_debug() {
    let m = M::up_with_hook("", |_: &Transaction| Ok(()));
    assert_eq!(
        format!(
            r#"M {{ up: "", up_hook: {:?}, down: None, down_hook: None, foreign_key_check: false, comment: None }}"#,
            m.up_hook
        ),
        format!("{m:?}")
    );
}

#[test]
fn test_schema_version_error_display() {
    let err = SchemaVersionError::TargetVersionOutOfRange {
        specified: SchemaVersion::NoneSet,
        highest: SchemaVersion::NoneSet,
    };
    assert_eq!("Attempt to migrate to version 0 (no version set), which is higher than the highest version currently supported, 0 (no version set).", format!("{err}"))
}

#[test]
fn test_foreign_key_check_error_display() {
    let err = ForeignKeyCheckError {
        table: "a".to_string(),
        rowid: 1,
        parent: "b".to_string(),
        fkid: 2,
    };
    assert_eq!("Foreign key check found row with id 1 in table 'a' missing from table 'b' but required by foreign key with id 2", format!("{err}"))
}

#[test]
fn test_migration_definition_error_display() {
    let err = MigrationDefinitionError::DownNotDefined { migration_index: 1 };
    assert_eq!(
        "Migration 1 (version 1 -> 2) cannot be reverted",
        format!("{err}")
    );

    let err = MigrationDefinitionError::DatabaseTooFarAhead;
    assert_eq!(
        "Attempt to migrate a database with a migration number that is too high",
        format!("{err}")
    );

    let err = MigrationDefinitionError::NoMigrationsDefined;
    assert_eq!(
        "Attempt to migrate with no migrations defined",
        format!("{err}")
    )
}

#[test]
fn test_error_display() {
    let err = Error::SpecifiedSchemaVersion(SchemaVersionError::TargetVersionOutOfRange {
        specified: SchemaVersion::NoneSet,
        highest: SchemaVersion::NoneSet,
    });
    assert_eq!(
        "rusqlite_migrate error: SpecifiedSchemaVersion(TargetVersionOutOfRange { specified: NoneSet, highest: NoneSet })",
        format!("{err}")
    );

    let err = Error::Hook(String::new());
    assert_eq!("rusqlite_migrate error: Hook(\"\")", format!("{err}"));

    let err = Error::ForeignKeyCheck(ForeignKeyCheckError {
        table: String::new(),
        rowid: 1,
        parent: String::new(),
        fkid: 2,
    });
    assert_eq!("rusqlite_migrate error: ForeignKeyCheck(ForeignKeyCheckError { table: \"\", rowid: 1, parent: \"\", fkid: 2 })", format!("{err}"));

    let err = Error::MigrationDefinition(MigrationDefinitionError::NoMigrationsDefined);
    assert_eq!(
        "rusqlite_migrate error: MigrationDefinition(NoMigrationsDefined)",
        format!("{err}")
    );

    let err = Error::RusqliteError {
        query: String::new(),
        err: rusqlite::Error::InvalidQuery,
    };
    assert_eq!(
        "rusqlite_migrate error: RusqliteError { query: \"\", err: InvalidQuery }",
        format!("{err}")
    );
}

#[test]
fn schema_version_partial_display_test() {
    assert_eq!("0 (no version set)", format!("{}", SchemaVersion::NoneSet));
    assert_eq!(
        "1 (inside)",
        format!("{}", SchemaVersion::Inside(NonZeroUsize::new(1).unwrap()))
    );
    assert_eq!(
        "32 (inside)",
        format!("{}", SchemaVersion::Inside(NonZeroUsize::new(32).unwrap()))
    );
    assert_eq!(
        "1 (outside)",
        format!("{}", SchemaVersion::Outside(NonZeroUsize::new(1).unwrap()))
    );
    assert_eq!(
        "32 (outside)",
        format!("{}", SchemaVersion::Outside(NonZeroUsize::new(32).unwrap()))
    );
}

#[test]
fn error_test_source() {
    let err = Error::RusqliteError {
        query: String::new(),
        err: rusqlite::Error::InvalidQuery,
    };
    assert_eq!(
        std::error::Error::source(&err)
            .and_then(|e| e.downcast_ref::<rusqlite::Error>())
            .unwrap(),
        &rusqlite::Error::InvalidQuery
    );

    let err = Error::SpecifiedSchemaVersion(SchemaVersionError::TargetVersionOutOfRange {
        specified: SchemaVersion::NoneSet,
        highest: SchemaVersion::NoneSet,
    });
    assert_eq!(
        std::error::Error::source(&err)
            .and_then(|e| e.downcast_ref::<SchemaVersionError>())
            .unwrap(),
        &SchemaVersionError::TargetVersionOutOfRange {
            specified: SchemaVersion::NoneSet,
            highest: SchemaVersion::NoneSet
        }
    );

    let err = Error::MigrationDefinition(MigrationDefinitionError::NoMigrationsDefined);
    assert_eq!(
        std::error::Error::source(&err)
            .and_then(|e| e.downcast_ref::<MigrationDefinitionError>())
            .unwrap(),
        &MigrationDefinitionError::NoMigrationsDefined
    );

    let err = Error::ForeignKeyCheck(ForeignKeyCheckError {
        table: String::new(),
        rowid: 1i64,
        parent: String::new(),
        fkid: 1i64,
    });
    assert_eq!(
        std::error::Error::source(&err)
            .and_then(|e| e.downcast_ref::<ForeignKeyCheckError>())
            .unwrap(),
        &ForeignKeyCheckError {
            table: String::new(),
            rowid: 1i64,
            parent: String::new(),
            fkid: 1i64,
        }
    );

    let err = Error::Hook(String::new());
    assert!(std::error::Error::source(&err).is_none());

    let err = Error::FileLoad(String::new());
    assert!(std::error::Error::source(&err).is_none());
}

#[test]
fn user_version_convert_test() {
    let mut conn = Connection::open_in_memory().unwrap();
    let migrations = Migrations::new(vec![m_valid10()]);
    assert_eq!(Ok(()), migrations.to_latest(&mut conn));
    assert_eq!(Ok(1), user_version(&conn));
    assert_eq!(
        Ok(SchemaVersion::Inside(NonZeroUsize::new(1).unwrap())),
        migrations.current_version(&conn)
    );
    assert_eq!(1usize, migrations.current_version(&conn).unwrap().into());
}

#[test]
fn user_version_migrate_test() {
    let mut conn = Connection::open_in_memory().unwrap();
    let migrations = Migrations::new(vec![m_valid10()]);

    assert_eq!(Ok(0), user_version(&conn));

    assert_eq!(Ok(()), migrations.to_latest(&mut conn));
    assert_eq!(Ok(1), user_version(&conn));
    assert_eq!(
        Ok(SchemaVersion::Inside(NonZeroUsize::new(1).unwrap())),
        migrations.current_version(&conn)
    );

    let migrations = Migrations::new(vec![m_valid10(), m_valid11()]);
    assert_eq!(Ok(()), migrations.to_latest(&mut conn));
    assert_eq!(Ok(2), user_version(&conn));
    assert_eq!(
        Ok(SchemaVersion::Inside(NonZeroUsize::new(2).unwrap())),
        migrations.current_version(&conn)
    );
}

#[test]
fn migration_partial_eq_test() {
    let m1 = M::up("");
    let m2 = M::up("");
    let m3 = M::up("TEST");

    assert_eq!(m1, m2);
    assert_ne!(m1, m3);
}

#[test]
fn user_version_start_0_test() {
    let conn = Connection::open_in_memory().unwrap();
    assert_eq!(Ok(0), user_version(&conn))
}

#[test]
fn invalid_migration_statement_test() {
    for m in &[m_invalid0(), m_invalid1(), m_valid11(), m_valid21()] {
        let migrations = Migrations::new(vec![m.clone()]);
        assert_ne!(Ok(()), migrations.validate())
    }
}

#[test]
fn invalid_migration_multiple_statement_test() {
    let migrations = Migrations::new(vec![m_valid0(), m_invalid1()]);
    assert!(matches!(
        dbg!(migrations.validate()),
        Err(Error::RusqliteError { query: _, err: _ })
    ));
}

#[test]
fn valid_migration_multiple_statement_test() {
    for m in &[m_valid0(), m_valid10(), m_valid20()] {
        let migrations = Migrations::new(vec![m.clone()]);
        assert_eq!(Ok(()), migrations.validate())
    }
}

#[test]
fn valid_fk_check_test() {
    assert_eq!(Ok(()), Migrations::new(vec![m_valid_fk()]).validate())
}

#[test]
fn invalid_fk_check_test() {
    let migrations = Migrations::new(vec![m_invalid_fk()]);
    assert!(matches!(
        dbg!(migrations.validate()),
        Err(Error::ForeignKeyCheck(_))
    ));
}

#[test]
fn all_valid_test() {
    assert_eq!(Ok(()), Migrations::new(all_valid()).validate());
}

// If we encounter a database with a migration number higher than the number of defined migration,
// we should return an error, not panic.
// See https://github.com/cljoly/rusqlite_migration/issues/17
#[test]
fn current_version_gt_max_schema_version_test() {
    let mut conn = Connection::open_in_memory().unwrap();

    // Set migrations to a higher number
    {
        let migrations = Migrations::new(vec![m_valid0(), m_valid10()]);
        migrations.to_latest(&mut conn).unwrap();
    }

    // We now have less migrations
    let migrations = Migrations::new(vec![m_valid0()]);

    // We should get an error
    assert_eq!(
        migrations.to_latest(&mut conn),
        Err(Error::MigrationDefinition(
            MigrationDefinitionError::DatabaseTooFarAhead
        ))
    );
}

#[test]
fn hook_test() {
    let mut conn = Connection::open_in_memory().unwrap();

    let text = "Lorem ipsum dolor sit amet, consectetur adipisici elit …".to_string();
    let cloned = text.clone();

    let migrations = Migrations::new(vec![
        M::up_with_hook(
            "CREATE TABLE novels (text TEXT);",
            move |tx: &Transaction| {
                tx.execute("INSERT INTO novels (text) VALUES (?1)", (&cloned,))?;
                Ok(())
            },
        ),
        M::up_with_hook(
            "ALTER TABLE novels ADD compressed TEXT;",
            |tx: &Transaction| {
                let mut stmt = tx.prepare("SELECT rowid, text FROM novels").unwrap();
                let rows = stmt.query_map([], |row| {
                    Ok((row.get_unwrap::<_, i64>(0), row.get_unwrap::<_, String>(1)))
                })?;

                for row in rows {
                    let row = row.unwrap();
                    let rowid = row.0;
                    let text = row.1;
                    let compressed = &text[..text.len() / 2];
                    tx.execute(
                        "UPDATE novels SET compressed = ?1 WHERE rowid = ?2;",
                        rusqlite::params![compressed, rowid],
                    )?;
                }

                Ok(())
            },
        )
        .down_with_hook(
            "ALTER TABLE novels DROP COLUMN compressed",
            |_: &Transaction| Ok(()),
        ),
    ]);

    assert_eq!(Ok(()), migrations.to_version(&mut conn, 2));

    let result: (String, String) = conn
        .query_row(
            "SELECT text, compressed FROM novels WHERE rowid = 1",
            [],
            |row| Ok((row.get(0).unwrap(), row.get(1).unwrap())),
        )
        .unwrap();

    assert_eq!(result.0, text);
    assert!(text.starts_with(&result.1));

    assert_eq!(Ok(()), migrations.to_version(&mut conn, 1));
}

#[test]
fn eq_hook_test() {
    let vec_migrations = vec![
        M::up("CREATE TABLE novels (text TEXT);"),
        // Different up
        M::up("CREATE TABLE IF NOT EXISTS novels (text TEXT);"),
        // Same up, different down
        M::up("CREATE TABLE IF NOT EXISTS novels (text TEXT);").down("DROP TABLE novels;"),
        // Use hooks now
        M::up_with_hook(
            "ALTER TABLE novels ADD compressed TEXT;",
            |_: &Transaction| Ok(()),
        )
        .down_with_hook(
            "ALTER TABLE novels DROP COLUMN compressed",
            |_: &Transaction| Ok(()),
        ),
        // Same as above, but different closures
        M::up_with_hook(
            "ALTER TABLE novels ADD compressed TEXT;",
            |_: &Transaction| Ok(()),
        )
        .down_with_hook(
            "ALTER TABLE novels DROP COLUMN compressed",
            |_: &Transaction| Ok(()),
        ),
        // Only with down hooks
        M::up_with_hook(
            "ALTER TABLE novels ADD compressed TEXT;",
            |_: &Transaction| Ok(()),
        )
        .down_with_hook(
            "ALTER TABLE novels DROP COLUMN compressed",
            |_: &Transaction| Ok(()),
        ),
        // Same as above, the closure should be deemed different
        M::up_with_hook(
            "ALTER TABLE novels ADD compressed TEXT;",
            |_: &Transaction| Ok(()),
        )
        .down_with_hook(
            "ALTER TABLE novels DROP COLUMN compressed",
            |_: &Transaction| Ok(()),
        ),
    ];
    // When there are no hooks, migrations can be cloned and still be equal
    {
        let migrations = Migrations::new_iter(vec_migrations.clone().into_iter().take(2));

        assert_eq!(migrations, migrations.clone());
    }

    // Complementary checks that PartialEq works as expected. We use assert_{eq,ne} to make
    // debugging easier
    for i in 0..vec_migrations.len() {
        for j in 0..vec_migrations.len() {
            if i == j {
                assert_eq!(&vec_migrations[i], &vec_migrations[j]);
                continue;
            }
            assert_ne!(&vec_migrations[i], &vec_migrations[j]);
        }
    }
    assert_eq!(&vec_migrations[1], &vec_migrations[1]);
    assert_ne!(&vec_migrations[0], &vec_migrations[1]);
}

#[test]
fn test_from_iter() {
    let migrations = Migrations::from_iter(vec![m_valid0(), m_valid10()]);
    assert_eq!(Ok(()), migrations.validate());
}
