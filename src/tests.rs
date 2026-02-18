use crate as tism;

#[test]
fn test_create() {
    let mut shm = tism::create("test_create_shm", 0).unwrap();
    assert_eq!(0, shm.read().unwrap());
    shm.write(1024).unwrap();
    assert_eq!(1024, shm.read().unwrap());
    assert_eq!(size_of::<i32>(), shm.allocated_data_size());
}

#[test]
fn test_open() {
    let mut owner = tism::create("test_open_shm", 0).unwrap();
    let mut borrower = tism::open::<i32>("test_open_shm").unwrap();
    assert_eq!(0, borrower.read().unwrap());
    owner.write(1024).unwrap();
    assert_eq!(1024, borrower.read().unwrap());
    assert_eq!(size_of::<i32>(), owner.allocated_data_size());
    assert_eq!(size_of::<i32>(), borrower.allocated_data_size());
}

#[test]
fn test_read_lock() {
    let mut owner = tism::create("test_read_lock_shm", 0).unwrap();
    let mut borrower = tism::open::<i32>("test_read_lock_shm").unwrap();

    let rl = borrower.read_lock().unwrap();
    assert_eq!(0, *rl.as_ref());
    std::mem::drop(rl);

    owner.write(1024).unwrap();

    let rl = borrower.read_lock().unwrap();
    assert_eq!(1024, *rl.as_ref());
    std::mem::drop(rl);

    assert_eq!(size_of::<i32>(), owner.allocated_data_size());
    assert_eq!(size_of::<i32>(), borrower.allocated_data_size());
}

#[test]
fn test_write_lock() {
    let mut owner = tism::create("test_write_lock_shm", 0).unwrap();
    let mut borrower = tism::open::<i32>("test_write_lock_shm").unwrap();

    assert_eq!(0, borrower.read().unwrap());

    let mut wl = owner.write_lock().unwrap();
    *wl.as_mut() = 1024;
    std::mem::drop(wl);

    assert_eq!(1024, borrower.read().unwrap());

    // Will hang unless out drop works correctly and unlocks
    let mut wl = owner.write_lock().unwrap();
    *wl.as_mut() = 33333;
    std::mem::drop(wl);

    assert_eq!(33333, borrower.read().unwrap());

    assert_eq!(size_of::<i32>(), owner.allocated_data_size());
    assert_eq!(size_of::<i32>(), borrower.allocated_data_size());
}
