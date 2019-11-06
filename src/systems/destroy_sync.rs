use super::*;

pub struct DestroySync {
    reader: ReaderId<InsertEvent>,
}

impl DestroySync {
    pub fn new(reader: ReaderId<InsertEvent>) -> Self {
        DestroySync { reader: reader }
    }
}

impl<'a> System<'a> for DestroySync {
    type SystemData = (
        Write<'a, EventChannel<InsertEvent>>,
        ReadExpect<'a, Arc<Mutex<EventChannel<InsertEvent>>>>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut insert_channel, asteroid_channel) = data;
        for insert in asteroid_channel.lock().unwrap().read(&mut self.reader) {
            insert_channel.single_write(insert.clone());
        }
    }
}
