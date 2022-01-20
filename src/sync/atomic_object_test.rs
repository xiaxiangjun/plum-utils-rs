use crate::sync::AtomicObject;

struct Object {
    flag: &'static str,
}

impl Object {
    fn new(flag: &'static str) -> Self {
        println!(">>> new: {}", flag);
        Object { flag }
    }
}

impl Drop for Object {
    fn drop(&mut self) {
        println!(">>> drop: {}", self.flag);
    }
}

#[test]
fn atomic_object_test() {
    let o = Object::new("test-1");
    drop(o);
    println!("----------: {}", line!());

    let o = Object::new("test-2");
    let ao = AtomicObject::default();
    ao.store(o);
    println!("----------: {}", line!());

    let o = Object::new("test-3");
    ao.store(o);
    println!("----------: {}", line!());

    let o = Object::new("test-4");
    ao.store(o);
    drop(ao);
    println!("----------: {}", line!());

    let ao = AtomicObject::default();
    let o = Object::new("test-5");
    ao.store(o);
    println!("----------: {}", line!());

    let o = ao.load();
    drop(ao);
    println!("----------: {}", line!());

    drop(o);
    println!("----------: {}", line!());
}
