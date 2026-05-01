use crate::Address;
use std::collections::BTreeMap;

pub struct Module {
    pub base_address: Address,
    pub end_address: Address,
    pub entry_point: Option<Address>,
    pub name: String,
    pub path: String,
    pub size_of_image: usize,
}

impl Module {
    pub fn contains(&self, address: Address) -> bool {
        address.0 >= self.base_address.0 && address.0 < self.end_address.0
    }
}

pub struct ModulesManager {
    modules: BTreeMap<usize, Module>,
}

impl ModulesManager {
    pub fn new() -> Self {
        Self { modules: BTreeMap::new() }
    }

    pub fn insert(&mut self, module: Module) {
        self.modules.insert(module.base_address.0, module);
    }

    pub fn find_by_address(&self, address: Address) -> Option<&Module> {

        let entry = self.modules.range(..=address.0).next_back()?;
        let entry = self.modules.range(..=address.0).next_back()?;
        let (_, module) = entry;
        
        if address.0 < module.end_address.0 {
            Some(module)
        } else {
            None
        }
    }
    
    pub fn find_by_name(&self, name: &str) -> Option<&Module> {
        self.modules.values().find(|m| m.name == name)
    }
}