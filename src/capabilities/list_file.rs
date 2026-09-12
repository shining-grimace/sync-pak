/// Platform document access for portable connection-list JSON.
pub trait ListFilePicker {
    fn open(&self, completion: ListFileCompletion) -> Result<(), String>;
    fn save(&self, contents: Vec<u8>, completion: ListFileCompletion) -> Result<(), String>;
}

/// None is cancellation; successful saves return an empty byte vector.
pub type ListFileCompletion = Box<dyn FnOnce(Result<Option<Vec<u8>>, String>) + Send + 'static>;
