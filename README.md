# TOM Interface for Shared Memory

A safe wrapper over Unix and Unix-like shared memory APIs. The `tism` API is cleanest in Rust, where its been designed to work nicely with the borrow checker and leverage it to our advantage in order to prevent bad usage of the shared memory.

If best practices are followed `tism` will be perfectly safe in C and Python as well, but missuse in these languages is slightly easier than in Rust. To make sure you're using `tism` correctly see the examples for each language and read the relevant documentation.

`tism` allows, for each allocation of shared memory, one publisher and any number of consumers. Consumers cannot read an allocation if a publisher is writing to it, and publishers cannot write to memory which is being read by consumers. You may read from shared memory with multiple consumers at once, but only one publisher may write at a time (which is all that should exist for a given allocation anyways).

# Rust

## Including in Your Rust Project

Since this crate isn't on [crates.io](https://crates.io) you will need to include it in your project locally, here's how you can do that.

In `Cargo.toml`, in the `[dependencies]` section, add this line:

```toml
tism = { path = "../tism" }
```

This assumes that your project and the `tism` crate are in the same directory, you can adjust the path if you would like to move `tism` to another directory relative to your project.

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

Using the same style of interacting with our memory as with the publisher we can access our memory as a consumer in a read-only fashion. Lets suppose the publisher example above doesn't exit before we start out consumer here, then we can open the shared memory it created.

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

# C

The C API does not have the same automatic cleanup as Rust, as a result it only safely supports reading and writing via cloning the memory. If this is undesireable there are "unsafe" function which give manual control over the locks and give back a pointer to the shared memory. If you opt for the unsafe function, which is a valid choice, be _extremely_ careful about your use of the lock, since you can very easily prevent other processes from reading/writing that allocation.

## Including in Your Project

No matter what you do you'll need the header file, located at `tism-c/tism.h`. For TISM the header file will serve as the documentation, so please reference it for specifics on functions (expecially if you opt for manual control over the locks).

From here you could do a few things, the easiest is just including the source file `tism-c/tism.c` in your project, either in your source directory or more cleanly in a directory meant for dependencies. Alternatively you could precompile it, though for a project as lightweight as TISM this probably isn't worth the trouble.

## Using TISM-C

The basic safe workflow mimics the lesser recommended Rust workflow, and for C it is what I recommend you start with.

Be careful with what functions you use, C will automatically cast pointers to pointers to other types, making it easy to use the wrong function by mistake.

### As a Publisher

Some notes on error management here, all TISM functions return a `tism_result_t`, this should be checked for if it is `TISM_OK` or not. In calls to TISM functions here I used a macro `TISM_MBIND`, all this macro does is early return on a non `TISM_OK` return value. Here it would return the TISM error code and you could check that with your shell. This isn't necessarily the best way of error handling, but for propogating errors its very concise and I've come to like it. The usual `if` statements would do just fine too.

```c
#include "tism.h"
#include <assert.h>

int main() {
    int data = 0;
    tism_owned_shared_memory_t shm;

    /*
     * `data` is cloned by TISM into the new allocation.
     */

    TISM_MBIND(tism_create(&shm, "my_shm", &data, sizeof data));

    /*
     * TISM remembers the size from when you create the allocation,
     * so you don't need it from now on.
     */

    int read_data;
    TISM_MBIND(tism_owned_read(&shm, &read_data));

    assert(read_data == 0);

    int write_data = 12;
    TISM_MBIND(tism_owned_write(&shm, &write_data));

    TISM_MBIND(tism_owned_read(&shm, &read_data));

    assert(read_data == 12);

    TISM_MBIND(tism_owned_close(&shm));
}

```

### As a Consumer

Lets pretent we didn't call `tism_owned_close` in out publisher code. Since we left off our shared memory as being set to twelve with out publisher, lets check that on out consumer.

```c
#include "tism.h"
#include <assert.h>

int main() {
    tism_borrowed_shared_memory_t shm;

    /*
     * Consumers don't initialize allocations with a value.
     */

    TISM_MBIND(tism_open(&shm, "my_shm", sizeof(int)));

    /*
     * TISM remembers the size from when you create the allocation,
     * so you don't need it from now on.
     */

    int read_data;
    TISM_MBIND(tism_borrowed_read(&shm, &read_data));

    assert(read_data == 12);

    TISM_MBIND(tism_borrowed_close(&shm));
}

```

### With the "Unsafe" API

```c
#include "tism.h"
#include <assert.h>

typedef struct {
    int field_1;
    int field_2;
} my_data_t;

int main() {
    my_data_t my_data = {
        .field_1 = 12,
        .field_2 = 49,
    };
    
    tism_owned_shared_memory_t shm;

    TISM_MBIND(tism_create(&shm, "my_shm", &my_data, sizeof my_data));

    /* Safety third kids ;) */

    my_data_t* shm_data;

    /* TISM will give us a pointer to the shared memory */
    TISM_MBIND(tism_unsafe_owned_write_lock(&shm, (void**)&shm_data));
    shm_data->field_1 = 56;
    TISM_MBIND(tism_unsafe_owned_unlock(&shm, (void**)&shm_data));

    /*
     * When you pass the pointer to the unlock function it sets it to `NULL` for
     * a little more safety, you can however pass `NULL` in its place to ommit
     * this. I'll do that next to to show you.
     */

    assert(shm_data == NULL);

    /* Return to the danger zone >:3 */
    TISM_MBIND(tism_unsafe_owned_read_lock(&shm, (void**)&shm_data));

    /* The unsafe API lets you extract, effeciently, single fields like this. */
    int new_field_1 = shm_data->field_1;
    TISM_MBIND(tism_unsafe_owned_unlock(&shm, NULL));

    /*
     * Now, if you pass NULL to the unlock function, you need to pinky promise
     * TISM that you wont use the pointer, otherwise bad things could happen.
     */

    assert(new_field_1 == 56);

    TISM_MBIND(tism_owned_close(&shm));
}
```

# Python

## Including in Your Project

This ones a bit weird. You'll need to clone this repo, then with `tism/tism-py` as your active directory you'll need to run `tism_compile.py` (this file depends on `cffi` and `setuptools`). That script will generate a few files which have the preffix `_tism`, copy these as well as the `tism.py` file into your project. From here you can import and use the `tism.py` file as though it were native Python, and `tism.py` will perform the C foriegn function interface calls on your behalf.

If you get an error while running `tism_compile.py`, you might be missing a Python development package. On Ubuntu you can install it with:

```bash
apt install python3-dev
```

## Using TISM-Py

TISM-Py reguires that users of the module serialize their data to bytes to write and deserialize from bytes to read.

### As a Publisher

```py
import tism

if __name__ == "__main__":
    data = bytes([0xFF, 0x00])

    with create("my_shm", data) as shm:
        read_data = shm.read()

        print(f"{data} should be the same as {read_data}")

        shm.write(bytes([0xBE, 0xEF]))
        read_data = shm.read()

        print(f"Now we have BEEF: {read_data}")

    # closes automatically when the with-block ends
```

### As a Consumer

```py
import tism

if __name__ == "__main__":
    # our data earlier was two bytes
    with open("my_shm", 2) as shm:
        read_data = shm.read()
        print(f"{read_data} should be BEEF")

    # closes automatically when the with-block ends
```

