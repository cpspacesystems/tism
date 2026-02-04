# TOM Interface for Shared Memory

A safe wrapper over Unix and Unix like shared memory APIs. The `tism` API is cleanest in Rust, where its been designed to work nicely with the borrow checker and leverage it to our advantage in order to prevent bad usage of the shared memory.

If best practices are followed `tism` will be perfectly safe in C and Python as well, but missuse in these languages is easier than in Rust. To make sure you're using `tism` correctly see the examples for each language and read the relevant documentation.

`tism` allows, for each allocation of shared memory, one publisher and any number of consumers. Consumers cannot read an allocation if a publisher is writing to it, and publishers cannot write to memory which is being read by consumers. You may read from shared memory with multiple consumers at once, but only one publisher at a time (which is all that should exist for a given allocation anyways).

# Rust

## Including in Your Rust Project

Since this crate isn't on [crates.io](https://crates.io) you will need to include it in your project locally, here's how you can do that.

In `Cargo.toml`, in the `[dependencies]` section, add this line:

```toml
tism = { path = "../tism" }
```

This assumes that your project and the `tism` crate are in the same directory, you can adjust the path if you would like to move `tism` relative to your project.

## Using `tism` in Your Rust Project

Before getting to examples, lets make sure you can read the documentation. With `tism` as your current directory run this command:

```bash
$ cargo doc
```

And now you can open the file `target/doc/tism/index.html` to view `tism`'s Rust API's documentation.

### As a Publishing Process

There are other ways to use `tism`, I recommend this way as it provides the best possible performance without demanding any extra care from the programmer. To use another method see the Rust crate's documentation.

```rust
let mut my_shm = tism::create("my_shared_memory", 0).unwrap();

if let Ok(mut lock) = my_shm.write_lock() {
    // Now we can treat `lock` as a smart pointer via its `AsRef` and `AsMut`
    // implementations!

    let x = *lock.as_ref();
    *lock.as_mut() = x + 1;

    assert_eq!(lock.as_ref(), &1);
}

// When our lock drops out of scope, it unlocks for you!
```

### As a Consuming Process

Using the same style of interacting with our memory as with the publisher we can access our memory as a consumer, in a read-only fashion. Lets suppose the publisher example above doesn't exit before we start out consumer here, then we can open the shared memory it created.

```rust
let mut my_shm = tism::open::<i32>("my_shared_memory").unwrap();

if let Ok(lock) = my_shm.read_lock() {
    let x = *lock.as_ref();

    // Since we set our value to 1 in the publisher example, we expect it to be
    // 1 down here too.
    assert_eq!(lock.as_ref(), &1);
}

// As with the publisher, the consumer releases its lock when it drops.
```
