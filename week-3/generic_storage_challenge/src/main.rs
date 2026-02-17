use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use wincode::{SchemaRead, SchemaWrite};

use generic_storage_challenge::{
    borsh_serializer::BorshSerializer, json_serializer::JsonSerializer,
    wincode_serializer::WincodeSerializer, Storage,
};

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
struct Person {
    name: String,
    age: u32,
}

fn main() {
    let person = Person {
        name: "André".to_string(),
        age: 30,
    };

    println!("Original: {:?}\n", person);
    // borsh
    let mut borsh_storage = Storage::new(BorshSerializer);
    assert!(!borsh_storage.has_data());

    borsh_storage.save(&person).unwrap();
    assert!(borsh_storage.has_data());

    let loaded = borsh_storage.load().unwrap();
    println!("Borsh   loaded: {:?}", loaded);
    assert_eq!(person, loaded);

    // Wincode
    let mut wincode_storage = Storage::new(WincodeSerializer);
    wincode_storage.save(&person).unwrap();

    let loaded = wincode_storage.load().unwrap();
    println!("Wincode loaded: {:?}", loaded);
    assert_eq!(person, loaded);

    //  JSON
    let mut json_storage = Storage::new(JsonSerializer);
    json_storage.save(&person).unwrap();

    let loaded = json_storage.load().unwrap();
    println!("JSON    loaded: {:?}", loaded);
    assert_eq!(person, loaded);

    println!("\n✅ All serializers work correctly!");
}

// Unit tests

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_person() -> Person {
        Person {
            name: "Bob".to_string(),
            age: 25,
        }
    }

    #[test]
    fn test_borsh_save_load() {
        let person = sample_person();
        let mut storage = Storage::new(BorshSerializer);

        assert!(!storage.has_data());
        storage.save(&person).unwrap();
        assert!(storage.has_data());

        let loaded = storage.load().unwrap();
        assert_eq!(person, loaded);
    }

    #[test]
    fn test_wincode_save_load() {
        let person = sample_person();
        let mut storage = Storage::new(WincodeSerializer);

        storage.save(&person).unwrap();
        let loaded = storage.load().unwrap();
        assert_eq!(person, loaded);
    }

    #[test]
    fn test_json_save_load() {
        let person = sample_person();
        let mut storage = Storage::new(JsonSerializer);

        storage.save(&person).unwrap();
        let loaded = storage.load().unwrap();
        assert_eq!(person, loaded);
    }

    #[test]
    fn test_no_data_returns_error() {
        let storage: Storage<Person, BorshSerializer> = Storage::new(BorshSerializer);
        assert!(storage.load().is_err());
    }
}
