use idb::DatabaseEvent;

#[derive(Debug, Clone)]
pub struct IdbPersister {
    db: std::rc::Rc<idb::Database>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct IdbChangeSet {
    pub tip: u32,
    pub change_set: bdk_wallet::ChangeSet,
}

impl IdbPersister {
    pub async fn find_change_set(
        &self,
    ) -> Result<bdk_wallet::ChangeSet, web_sys::wasm_bindgen::JsValue> {
        let tx = self
            .db
            .transaction(&["change_sets"], idb::TransactionMode::ReadOnly)
            .map_err(|e| {
                web_sys::wasm_bindgen::JsValue::from_str(&format!(
                    "Failed to create transaction: {e}"
                ))
            })?;
        let store = tx.object_store("change_sets").map_err(|e| {
            web_sys::wasm_bindgen::JsValue::from_str(&format!("Failed to get object store: {e}"))
        })?;

        let change_sets: Vec<web_sys::wasm_bindgen::JsValue> = store
            .get_all(None, None)
            .map_err(|e| {
                web_sys::wasm_bindgen::JsValue::from_str(&format!(
                    "Failed to get all change sets: {e}"
                ))
            })?
            .await
            .map_err(|e| {
                web_sys::wasm_bindgen::JsValue::from_str(&format!("Failed to await get_all: {e}"))
            })?;

        let change_sets: Vec<bdk_wallet::ChangeSet> = change_sets
            .into_iter()
            .filter_map(|value| {
                let idb_change_set: Result<IdbChangeSet, _> = serde_wasm_bindgen::from_value(value);
                Some(idb_change_set.ok()?.change_set)
            })
            .collect();
        if change_sets.is_empty() {
            return Err(web_sys::wasm_bindgen::JsValue::from_str(
                "No change sets found",
            ));
        }
        let mut result = change_sets.first().cloned().unwrap();
        for change_set in change_sets.iter().skip(1) {
            bdk_wallet::chain::Merge::merge(&mut result, change_set.clone());
        }
        Ok(result)
    }
    pub async fn persist_change_set(
        &self,
        change_set: bdk_wallet::ChangeSet,
    ) -> Result<(), web_sys::wasm_bindgen::JsValue> {
        let tx = self
            .db
            .transaction(&["change_sets"], idb::TransactionMode::ReadWrite)
            .map_err(|e| {
                web_sys::wasm_bindgen::JsValue::from_str(&format!(
                    "Failed to create transaction: {e}"
                ))
            })?;
        let store = tx.object_store("change_sets").map_err(|e| {
            web_sys::wasm_bindgen::JsValue::from_str(&format!("Failed to get object store: {e}"))
        })?;

        let idb_change_set = IdbChangeSet {
            tip: change_set
                .local_chain
                .blocks
                .last_key_value()
                .map(|(height, _)| *height)
                .unwrap_or(0),
            change_set,
        };
        store
            .add(
                &serde_wasm_bindgen::to_value(&idb_change_set).unwrap(),
                None,
            )
            .map_err(|e| {
                web_sys::wasm_bindgen::JsValue::from_str(&format!("Failed to add change set: {e}"))
            })?;
        tx.commit().map_err(|e| {
            web_sys::wasm_bindgen::JsValue::from_str(&format!("Failed to commit transaction: {e}"))
        })?;
        Ok(())
    }
    pub async fn new() -> Option<Self> {
        let factory = idb::Factory::new().expect("Failed to create IDB factory");

        // Create an open request for the database
        let mut open_request = factory.open("test", Some(2)).expect("Failed to open IDB");

        // Set up the upgrade needed event
        open_request.on_upgrade_needed(|event| {
            // Get database instance from event
            let database = event.database().expect("Failed to get database from event");

            // Prepare object store params
            let mut store_params = idb::ObjectStoreParams::new();
            store_params.auto_increment(true);
            store_params.key_path(Some(idb::KeyPath::new_single("tip")));

            // Create object store
            let _store = database
                .create_object_store("updates", store_params)
                .expect("Failed to create object store");

            // Create another object store for change sets
            let mut change_set_params = idb::ObjectStoreParams::new();
            change_set_params.auto_increment(true);
            change_set_params.key_path(Some(idb::KeyPath::new_single("tip")));
            let _change_set_store = database
                .create_object_store("change_sets", change_set_params)
                .expect("Failed to create change sets object store");
        });

        let db = open_request.await.expect("Failed to open IDB database");
        Some(IdbPersister {
            db: std::rc::Rc::new(db),
        })
    }
}
