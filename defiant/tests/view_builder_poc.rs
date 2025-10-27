//! Proof of concept for View + Builder (Message) pattern

use defiant::Arena;

// User's struct - the View (immutable)
struct Person<'arena> {
    name: &'arena str,
    age: i32,
}

// Generated builder - PersonMessage (mutable)
struct PersonMessage<'arena> {
    arena: &'arena Arena,
    name: &'arena str,
    age: i32,
}

impl<'arena> PersonMessage<'arena> {
    pub fn new_in(arena: &'arena Arena) -> Self {
        Self {
            arena,
            name: "",
            age: 0,
        }
    }

    // Setters
    pub fn set_name(&mut self, value: &str) {
        self.name = self.arena.alloc_str(value);
    }

    pub fn set_age(&mut self, value: i32) {
        self.age = value;
    }

    // Getters
    pub fn name(&self) -> &str {
        self.name
    }

    pub fn age(&self) -> i32 {
        self.age
    }

    // Get view
    pub fn as_view(&self) -> Person<'arena> {
        Person {
            name: self.name,
            age: self.age,
        }
    }
}

#[test]
fn test_view_builder_pattern() {
    let arena = Arena::new();

    // Build using PersonMessage
    let mut msg = PersonMessage::new_in(&arena);
    msg.set_name("Alice");
    msg.set_age(30);

    // Get immutable view
    let view = msg.as_view();
    assert_eq!(view.name, "Alice");
    assert_eq!(view.age, 30);

    // Can still modify message
    msg.set_name("Bob");
    assert_eq!(msg.name(), "Bob");

    // Get updated view
    let view2 = msg.as_view();
    assert_eq!(view2.name, "Bob");
}
