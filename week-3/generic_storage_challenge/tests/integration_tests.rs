use borsh::{BorshDeserialize, BorshSerialize};
use generic_storage_challenge::borsh_serializer::BorshSerializer;
use generic_storage_challenge::json_serializer::JsonSerializer;
use generic_storage_challenge::wincode_serializer::WincodeSerializer;
use generic_storage_challenge::Storage;
use serde::{Deserialize, Serialize};
use wincode::{SchemaRead, SchemaWrite};

#[derive(
    Debug,
    PartialEq,
    Clone,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    SchemaWrite,
    SchemaRead,
)]
struct TestPerson {
    name: String,
    age: u32,
}

#[test]
fn test_conversion() {
    let person = TestPerson {
        name: "Andre".to_string(),
        age: 30,
    };

    // Starting with Borsh
    let mut storage = Storage::new(BorshSerializer);
    storage.save(&person).unwrap();

    // Convert to Json
    let json_storage = storage.convert(JsonSerializer).unwrap();
    let loaded_person = json_storage.load().unwrap();
    assert_eq!(person, loaded_person);

    // Convert to Wincode
    let wincode_storage = json_storage.convert(WincodeSerializer).unwrap();
    let loaded_person = wincode_storage.load().unwrap();
    assert_eq!(person, loaded_person);

    // Convert back to Borsh
    let borsh_storage = wincode_storage.convert(BorshSerializer).unwrap();
    let loaded_person = borsh_storage.load().unwrap();
    assert_eq!(person, loaded_person);
}

#[test]
fn test_borsh_save_load() {
    let person = TestPerson {
        name: "Andre".to_string(),
        age: 32,
    };
    let mut storage = Storage::new(BorshSerializer);

    assert!(!storage.has_data());
    storage.save(&person).unwrap();
    assert!(storage.has_data());

    let loaded = storage.load().unwrap();
    assert_eq!(person, loaded);
}

#[test]
fn test_wincode_save_load() {
    let person = TestPerson {
        name: "Andre".to_string(),
        age: 32,
    };
    let mut storage = Storage::new(WincodeSerializer);

    storage.save(&person).unwrap();
    let loaded = storage.load().unwrap();
    assert_eq!(person, loaded);
}

#[test]
fn test_json_save_load() {
    let person = TestPerson {
        name: "Andre".to_string(),
        age: 32,
    };
    let mut storage = Storage::new(JsonSerializer);

    storage.save(&person).unwrap();
    let loaded = storage.load().unwrap();
    assert_eq!(person, loaded);
}

#[test]
fn test_no_data_returns_error() {
    let storage: Storage<TestPerson, BorshSerializer> = Storage::new(BorshSerializer);
    assert!(storage.load().is_err());
}
