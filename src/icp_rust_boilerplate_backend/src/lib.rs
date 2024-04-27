#[ic_cdk::query]
fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}

#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Equipment {
    id: u64,
    name: String,
    description: String,
    rental_price: u64,
    created_at: u64,
    updated_at: Option<u64>,
}

impl Storable for Equipment {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Equipment {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static STORAGE: RefCell<StableBTreeMap<u64, Equipment, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct EquipmentPayload {
    name: String,
    description: String,
    rental_price: u64,
}

#[ic_cdk::query]
fn get_equipment(id: u64) -> Result<Equipment, Error> {
    match _get_equipment(&id) {
        Some(equipment) => Ok(equipment),
        None => Err(Error::NotFound {
            msg: format!("an equipment with id={} not found", id),
        }),
    }
}

#[ic_cdk::update]
fn add_equipment(equipment: EquipmentPayload) -> Option<Equipment> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");
    let equipment = Equipment {
        id,
        name: equipment.name,
        description: equipment.description,
        rental_price: equipment.rental_price,
        created_at: time(),
        updated_at: None,
    };
    do_insert(&equipment);
    Some(equipment)
}

#[ic_cdk::update]
fn update_equipment(id: u64, payload: EquipmentPayload) -> Result<Equipment, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut equipment) => {
            equipment.description = payload.description;
            equipment.name = payload.name;
            equipment.rental_price = payload.rental_price;
            equipment.updated_at = Some(time());
            do_insert(&equipment);
            Ok(equipment)
        }
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't update an equipment with id={}. equipment not found",
                id
            ),
        }),
    }
}

// helper method to perform insert.
fn do_insert(equipment: &Equipment) {
    STORAGE.with(|service| service.borrow_mut().insert(equipment.id, equipment.clone()));
}

#[ic_cdk::update]
fn delete_equipment(id: u64) -> Result<Equipment, Error> {
    match STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(equipment) => Ok(equipment),
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't delete an equipment with id={}. equipment not found.",
                id
            ),
        }),
    }
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
}

// a helper method to get an equipment by id. used in get_equipment/update_equipment
fn _get_equipment(id: &u64) -> Option<Equipment> {
    STORAGE.with(|service| service.borrow().get(id))
}

// need this to generate candid
ic_cdk::export_candid!();