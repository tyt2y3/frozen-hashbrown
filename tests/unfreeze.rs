use anyhow::{Context, Result};
use frozen_hashbrown::FrozenHashMap;
use std::{
    collections::HashMap,
    fmt::{Debug, Write},
};

#[test]
fn unfreeze() -> Result<()> {
    let map: HashMap<char, i32> = [('a', 1), ('b', 2), ('c', 3), ('d', 4)]
        .into_iter()
        .collect();
    let snapshot = format!("{map:?}");
    println!("{snapshot}");

    let frozen = FrozenHashMap::construct(&map);
    std::mem::drop(map);
    println!("{frozen:?}");
    let frozen: Vec<u8> = frozen.store();

    let mut unfrozen = FrozenHashMap::load(&frozen).context("Failed to load")?;
    let unfrozen = unfrozen
        .reconstruct::<char, i32>()
        .context("Failed to reconstruct")?;
    let unfrozen_snapshot = format!("{unfrozen:?}");

    // even the "random" iteration order holds
    assert_eq!(snapshot, unfrozen_snapshot);

    Ok(())
}

#[test]
fn unfreeze_str() -> Result<()> {
    let map: HashMap<&str, &str> = [
        ("apple", "12"),
        ("banana", "22"),
        ("cherry", "32"),
        ("dragonfruit", "42"),
    ]
    .into_iter()
    .collect();
    let snapshot = format!("{map:?}");
    println!("{snapshot}");

    let frozen = FrozenHashMap::construct(&map);
    std::mem::drop(map);
    println!("{frozen:?}");
    let frozen: Vec<u8> = frozen.store();

    let mut unfrozen = FrozenHashMap::load(&frozen).context("Failed to load")?;
    let unfrozen = unfrozen
        .reconstruct::<&str, &str>()
        .context("Failed to reconstruct")?;
    let unfrozen_snapshot = format!("{unfrozen:?}");
    assert_eq!(snapshot, unfrozen_snapshot);

    Ok(())
}

#[test]
fn unfreeze_large_set() -> Result<()> {
    let map: HashMap<i32, ()> = (1..=10_000).map(|v| (v, ())).collect();

    let frozen = FrozenHashMap::construct(&map);
    std::mem::drop(map);
    println!("{frozen:?}");
    let frozen: Vec<u8> = frozen.store();

    let mut unfrozen = FrozenHashMap::load(&frozen).context("Failed to load")?;
    let unfrozen = unfrozen
        .reconstruct::<i32, ()>()
        .context("Failed to reconstruct")?;

    let sum: i32 = unfrozen.iter().map(|(v, _)| *v).sum();
    assert_eq!(sum, 10000 * 10001 / 2);

    Ok(())
}

#[test]
fn unfreeze_u128() -> Result<()> {
    let map: HashMap<u32, u128> = [
        (111, 111_111_111),
        (222, 222_222_222),
        (333, 333_333_333),
        (444, 444_444_444),
    ]
    .into_iter()
    .collect();
    let snapshot = format!("{map:?}");
    println!("{snapshot}");

    let frozen = FrozenHashMap::construct(&map);
    std::mem::drop(map);
    println!("{frozen:?}");
    let frozen: Vec<u8> = frozen.store();

    let mut unfrozen = FrozenHashMap::load(&frozen).context("Failed to load")?;
    let unfrozen = unfrozen
        .reconstruct::<u32, u128>()
        .context("Failed to reconstruct")?;
    let unfrozen_snapshot = format!("{unfrozen:?}");
    assert_eq!(snapshot, unfrozen_snapshot);

    Ok(())
}

#[test]
fn unfreeze_raw() -> Result<()> {
    let map: HashMap<char, i32> = [('a', 1), ('b', 2), ('c', 3), ('d', 4)]
        .into_iter()
        .collect();
    let snapshot = format!("{map:?}");
    println!("{snapshot}");

    let frozen = FrozenHashMap::construct(&map);
    std::mem::drop(map);
    println!("{frozen:?}");
    let frozen: Vec<u8> = frozen.store();

    let mut unfrozen = FrozenHashMap::load(&frozen).context("Failed to load")?;

    // it only matters that T has the same size as (K, V)
    let unfrozen = unfrozen
        .reconstruct::<[u8; std::mem::size_of::<(char, i32)>()], ()>()
        .context("Failed to reconstruct")?;

    let mut unfrozen_snapshot = format!("{{");
    for (i, (ptr, _)) in unfrozen.iter().enumerate() {
        let (key, val): &(char, i32) = unsafe { core::mem::transmute(ptr) };
        write!(
            unfrozen_snapshot,
            "{}{:?}: {:?}",
            if i > 0 { ", " } else { "" },
            key,
            val,
        )
        .unwrap();
    }
    write!(unfrozen_snapshot, "}}").unwrap();
    assert_eq!(snapshot, unfrozen_snapshot);

    Ok(())
}

fn unfreeze_raw_iter_generic<K: Debug, V: Debug>(map: HashMap<K, V>) -> Result<()> {
    let snapshot = format!("{map:?}");
    println!("{snapshot}");

    let frozen = FrozenHashMap::construct(&map);
    std::mem::drop(map);
    println!("{frozen:?}");
    let frozen: Vec<u8> = frozen.store();

    let unfrozen = FrozenHashMap::load(&frozen).context("Failed to load")?;

    let raw_iter = unfrozen.raw_iter().unwrap();

    let mut unfrozen_snapshot = format!("{{");
    for (i, ptr) in raw_iter.enumerate() {
        let (key, val): &(K, V) = unsafe { core::mem::transmute(ptr) };
        write!(
            unfrozen_snapshot,
            "{}{:?}: {:?}",
            if i > 0 { ", " } else { "" },
            key,
            val,
        )
        .unwrap();
    }
    write!(unfrozen_snapshot, "}}").unwrap();
    assert_eq!(snapshot, unfrozen_snapshot);

    Ok(())
}

#[test]
fn unfreeze_raw_iter_generic_1() {
    let map: HashMap<char, i32> = [('a', 1), ('b', 2), ('c', 3), ('d', 4)]
        .into_iter()
        .collect();
    unfreeze_raw_iter_generic(map).unwrap();
}

#[test]
fn unfreeze_raw_iter_generic_2() {
    let map: HashMap<char, i32> = [
        ('a', 1),
        ('b', 2),
        ('c', 3),
        ('d', 4),
        ('e', 5),
        ('f', 6),
        ('g', 7),
        ('h', 8),
    ]
    .into_iter()
    .collect();
    unfreeze_raw_iter_generic(map).unwrap();
}

#[test]
fn unfreeze_raw_iter_generic_3() {
    // this has weird alignment
    let map: HashMap<u8, i32> = [
        (b'a', 1),
        (b'b', 2),
        (b'c', 3),
        (b'd', 4),
        (b'e', 5),
        (b'f', 6),
        (b'g', 7),
        (b'h', 8),
    ]
    .into_iter()
    .collect();
    unfreeze_raw_iter_generic(map).unwrap();
}

#[test]
fn unfreeze_raw_iter_generic_4() {
    // this has weird-er alignment
    let map: HashMap<u8, (i64, i32)> = [
        (b'a', (-1, 1)),
        (b'b', (-2, 2)),
        (b'c', (-3, 3)),
        (b'd', (-4, 4)),
        (b'e', (-5, 5)),
        (b'f', (-6, 6)),
        (b'g', (-7, 7)),
        (b'h', (-8, 8)),
        (b'i', (-9, 9)),
    ]
    .into_iter()
    .collect();
    unfreeze_raw_iter_generic(map).unwrap();
}
