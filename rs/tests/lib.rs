
#![feature(collections)]

extern crate lsm;

use lsm::ICursor;

fn tid() -> String {
    // TODO use the rand crate
    fn bytes() -> std::io::Result<[u8;16]> {
        use std::fs::OpenOptions;
        let mut f = try!(OpenOptions::new()
                .read(true)
                .open("/dev/urandom"));
        let mut ba = [0;16];
        try!(lsm::utils::ReadFully(&mut f, &mut ba));
        Ok(ba)
    }

    fn to_hex_string(ba: &[u8]) -> String {
        let strs: Vec<String> = ba.iter()
            .map(|b| format!("{:02X}", b))
            .collect();
        strs.connect("")
    }

    let ba = bytes().unwrap();
    to_hex_string(&ba)
}

fn tempfile(base: &str) -> String {
    std::fs::create_dir("tmp");
    let file = "tmp/".to_string() + base + "_" + &tid();
    file
}

fn to_utf8(s : &str) -> Box<[u8]> {
    s.to_string().into_bytes().into_boxed_slice()
}

fn from_utf8(a: Box<[u8]>) -> String {
    //let k = csr.Key();
    //let k = std::str::from_utf8(&k).unwrap();
    let k = std::string::String::from_utf8(a.into_iter().map(|b| *b).collect()).unwrap();
    k
}

fn insert_pair_string_string(d: &mut std::collections::HashMap<Box<[u8]>,Box<[u8]>>, k:&str, v:&str) {
    d.insert(to_utf8(k), to_utf8(v));
}

fn insert_pair_string_blob(d: &mut std::collections::HashMap<Box<[u8]>,lsm::Blob>, k:&str, v:lsm::Blob) {
    d.insert(to_utf8(k), v);
}

fn count_keys_forward(csr: &mut lsm::LivingCursor) -> std::io::Result<usize> {
    let mut r = 0;
    try!(csr.First());
    while csr.IsValid() {
        r = r + 1;
        try!(csr.Next());
    }
    Ok(r)
}

fn count_keys_backward(csr: &mut lsm::LivingCursor) -> std::io::Result<usize> {
    let mut r = 0;
    try!(csr.Last());
    while csr.IsValid() {
        r = r + 1;
        try!(csr.Prev());
    }
    Ok(r)
}

fn ReadValue(b: lsm::Blob) -> std::io::Result<Box<[u8]>> {
    match b {
        lsm::Blob::Stream(mut strm) => {
            let mut a = Vec::new();
            try!(strm.read_to_end(&mut a));
            Ok(a.into_boxed_slice())
        },
        lsm::Blob::Array(a) => Ok(a),
        lsm::Blob::Tombstone => panic!(),
    }
}

#[test]
fn empty_cursor() {
    fn f() -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("empty_cursor"), lsm::DEFAULT_SETTINGS));
        let mut csr = try!(db.OpenCursor());
        try!(csr.First());
        assert!(!csr.IsValid());
        try!(csr.Last());
        assert!(!csr.IsValid());
        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn first_prev() {
    fn f() -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("first_prev"), lsm::DEFAULT_SETTINGS));
        let g = try!(db.WriteSegmentFromSortedSequence(lsm::GenerateNumbers {cur: 0, end: 100, step: 1}));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g]));
        }
        let mut csr = try!(db.OpenCursor());
        try!(csr.First());
        assert!(csr.IsValid());
        try!(csr.Prev());
        assert!(!csr.IsValid());
        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn last_next() {
    fn f() -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("first_prev"), lsm::DEFAULT_SETTINGS));
        let g = try!(db.WriteSegmentFromSortedSequence(lsm::GenerateNumbers {cur: 0, end: 100, step: 1}));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g]));
        }
        let mut csr = try!(db.OpenCursor());
        try!(csr.Last());
        assert!(csr.IsValid());
        try!(csr.Next());
        assert!(!csr.IsValid());
        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn seek() {
    fn f() -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("seek"), lsm::DEFAULT_SETTINGS));
        let g = try!(db.WriteSegmentFromSortedSequence(lsm::GenerateNumbers {cur: 0, end: 100, step: 1}));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g]));
        }
        let mut csr = try!(db.OpenCursor());
        try!(csr.First());
        assert!(csr.IsValid());
        // TODO constructing the utf8 byte array seems convoluted

        let k = format!("{:08}", 42).into_bytes().into_boxed_slice();
        try!(csr.Seek(&k, lsm::SeekOp::SEEK_EQ));
        assert!(csr.IsValid());

        let k = format!("{:08}", 105).into_bytes().into_boxed_slice();
        try!(csr.Seek(&k, lsm::SeekOp::SEEK_EQ));
        assert!(!csr.IsValid());

        let k = format!("{:08}", 105).into_bytes().into_boxed_slice();
        try!(csr.Seek(&k, lsm::SeekOp::SEEK_GE));
        assert!(!csr.IsValid());

        let k = format!("{:08}", 105).into_bytes().into_boxed_slice();
        try!(csr.Seek(&k, lsm::SeekOp::SEEK_LE));
        assert!(csr.IsValid());
        // TODO get the key

        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn lexographic() {
    fn f() -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("lexicographic"), lsm::DEFAULT_SETTINGS));
        let mut d = std::collections::HashMap::new();
        insert_pair_string_string(&mut d, "8", "");
        insert_pair_string_string(&mut d, "10", "");
        insert_pair_string_string(&mut d, "20", "");
        let g = try!(db.WriteSegment(d));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g]));
        }
        let mut csr = try!(db.OpenCursor());
        try!(csr.First());
        assert!(csr.IsValid());
        assert_eq!(from_utf8(csr.Key().unwrap()), "10");

        try!(csr.Next());
        assert!(csr.IsValid());
        assert_eq!(from_utf8(csr.Key().unwrap()), "20");

        try!(csr.Next());
        assert!(csr.IsValid());
        assert_eq!(from_utf8(csr.Key().unwrap()), "8");

        try!(csr.Next());
        assert!(!csr.IsValid());

        // --------
        try!(csr.Last());
        assert!(csr.IsValid());
        assert_eq!(from_utf8(csr.Key().unwrap()), "8");

        try!(csr.Prev());
        assert!(csr.IsValid());
        assert_eq!(from_utf8(csr.Key().unwrap()), "20");

        try!(csr.Prev());
        assert!(csr.IsValid());
        assert_eq!(from_utf8(csr.Key().unwrap()), "10");

        try!(csr.Prev());
        assert!(!csr.IsValid());

        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn seek_cur() {
    fn f() -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("seek_cur"), lsm::DEFAULT_SETTINGS));
        let mut t1 = std::collections::HashMap::new();
        for i in 0 .. 100 {
            let sk = format!("{:03}", i);
            let sv = format!("{}", i);
            insert_pair_string_string(&mut t1, &sk, &sv);
        }
        let mut t2 = std::collections::HashMap::new();
        for i in 0 .. 1000 {
            let sk = format!("{:05}", i);
            let sv = format!("{}", i);
            insert_pair_string_string(&mut t2, &sk, &sv);
        }
        let g1 = try!(db.WriteSegment(t1));
        let g2 = try!(db.WriteSegment(t2));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g1]));
            try!(lck.commitSegments(vec![g2]));
        }
        let mut csr = try!(db.OpenCursor());
        try!(csr.Seek(&to_utf8("00001"), lsm::SeekOp::SEEK_EQ));
        assert!(csr.IsValid());
        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn weird() {
    fn f() -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("weird"), lsm::DEFAULT_SETTINGS));
        let mut t1 = std::collections::HashMap::new();
        for i in 0 .. 100 {
            let sk = format!("{:03}", i);
            let sv = format!("{}", i);
            insert_pair_string_string(&mut t1, &sk, &sv);
        }
        let mut t2 = std::collections::HashMap::new();
        for i in 0 .. 1000 {
            let sk = format!("{:05}", i);
            let sv = format!("{}", i);
            insert_pair_string_string(&mut t2, &sk, &sv);
        }
        let g1 = try!(db.WriteSegment(t1));
        let g2 = try!(db.WriteSegment(t2));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g1]));
            try!(lck.commitSegments(vec![g2]));
        }
        let mut csr = try!(db.OpenCursor());
        try!(csr.First());
        for _ in 0 .. 100 {
            try!(csr.Next());
            assert!(csr.IsValid());
        }
        for _ in 0 .. 50 {
            try!(csr.Prev());
            assert!(csr.IsValid());
        }
        for _ in 0 .. 100 {
            try!(csr.Next());
            assert!(csr.IsValid());
            try!(csr.Next());
            assert!(csr.IsValid());
            try!(csr.Prev());
            assert!(csr.IsValid());
        }
        println!("{:?}", csr.Key());
        for _ in 0 .. 50 {
            let k = csr.Key().unwrap();
            println!("{:?}", k);
            try!(csr.Seek(&k, lsm::SeekOp::SEEK_EQ));
            assert!(csr.IsValid());
            try!(csr.Next());
            assert!(csr.IsValid());
        }
        for _ in 0 .. 50 {
            let k = csr.Key().unwrap();
            try!(csr.Seek(&k, lsm::SeekOp::SEEK_EQ));
            assert!(csr.IsValid());
            try!(csr.Prev());
            assert!(csr.IsValid());
        }
        for _ in 0 .. 50 {
            let k = csr.Key().unwrap();
            try!(csr.Seek(&k, lsm::SeekOp::SEEK_LE));
            assert!(csr.IsValid());
            try!(csr.Prev());
            assert!(csr.IsValid());
        }
        for _ in 0 .. 50 {
            let k = csr.Key().unwrap();
            try!(csr.Seek(&k, lsm::SeekOp::SEEK_GE));
            assert!(csr.IsValid());
            try!(csr.Next());
            assert!(csr.IsValid());
        }
        // got the following value from the debugger.
        // just want to make sure that it doesn't change
        // and all combos give the same answer.
        assert_eq!(from_utf8(csr.Key().unwrap()), "00148");
        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn no_le_ge_multicursor() {
    fn f() -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("no_le_ge_multicursor"), lsm::DEFAULT_SETTINGS));

        let mut t1 = std::collections::HashMap::new();
        insert_pair_string_string(&mut t1, "c", "3");
        insert_pair_string_string(&mut t1, "g", "7");
        let g1 = try!(db.WriteSegment(t1));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g1]));
        }

        let mut t2 = std::collections::HashMap::new();
        insert_pair_string_string(&mut t2, "e", "5");
        let g2 = try!(db.WriteSegment(t2));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g2]));
        }

        let mut csr = try!(db.OpenCursor());

        try!(csr.Seek(&to_utf8("a"), lsm::SeekOp::SEEK_LE));
        assert!(!csr.IsValid());

        try!(csr.Seek(&to_utf8("d"), lsm::SeekOp::SEEK_LE));
        assert!(csr.IsValid());

        try!(csr.Seek(&to_utf8("f"), lsm::SeekOp::SEEK_GE));
        assert!(csr.IsValid());

        try!(csr.Seek(&to_utf8("h"), lsm::SeekOp::SEEK_GE));
        assert!(!csr.IsValid());

        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn empty_val() {
    fn f() -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("empty_val"), lsm::DEFAULT_SETTINGS));

        let mut t1 = std::collections::HashMap::new();
        insert_pair_string_string(&mut t1, "_", "");
        let g1 = try!(db.WriteSegment(t1));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g1]));
        }
        let mut csr = try!(db.OpenCursor());
        try!(csr.Seek(&to_utf8("_"), lsm::SeekOp::SEEK_EQ));
        assert!(csr.IsValid());
        assert_eq!(0, csr.ValueLength().unwrap().unwrap());

        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn delete_not_there() {
    fn f() -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("delete_not_there"), lsm::DEFAULT_SETTINGS));

        let mut t1 = std::collections::HashMap::new();
        insert_pair_string_string(&mut t1, "a", "1");
        insert_pair_string_string(&mut t1, "b", "2");
        insert_pair_string_string(&mut t1, "c", "3");
        insert_pair_string_string(&mut t1, "d", "4");
        let g1 = try!(db.WriteSegment(t1));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g1]));
        }

        let mut t2 = std::collections::HashMap::new();
        insert_pair_string_blob(&mut t2, "e", lsm::Blob::Tombstone);
        let g2 = try!(db.WriteSegment2(t2));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g2]));
        }

        let mut csr = try!(db.OpenCursor());
        assert_eq!(4, try!(count_keys_forward(&mut csr)));
        assert_eq!(4, try!(count_keys_backward(&mut csr)));

        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn delete_nothing_there() {
    fn f() -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("delete_nothing_there"), lsm::DEFAULT_SETTINGS));

        let mut t2 = std::collections::HashMap::new();
        insert_pair_string_blob(&mut t2, "e", lsm::Blob::Tombstone);
        let g2 = try!(db.WriteSegment2(t2));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g2]));
        }

        let mut csr = try!(db.OpenCursor());
        assert_eq!(0, try!(count_keys_forward(&mut csr)));
        assert_eq!(0, try!(count_keys_backward(&mut csr)));

        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn simple_tombstone() {
    fn f(del: &str) -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("simple_tombstone"), lsm::DEFAULT_SETTINGS));

        let mut t1 = std::collections::HashMap::new();
        insert_pair_string_string(&mut t1, "a", "1");
        insert_pair_string_string(&mut t1, "b", "2");
        insert_pair_string_string(&mut t1, "c", "3");
        insert_pair_string_string(&mut t1, "d", "4");
        let g1 = try!(db.WriteSegment(t1));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g1]));
        }

        let mut t2 = std::collections::HashMap::new();
        insert_pair_string_blob(&mut t2, del, lsm::Blob::Tombstone);
        let g2 = try!(db.WriteSegment2(t2));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g2]));
        }

        let mut csr = try!(db.OpenCursor());
        assert_eq!(3, try!(count_keys_forward(&mut csr)));
        assert_eq!(3, try!(count_keys_backward(&mut csr)));

        Ok(())
    }
    assert!(f("a").is_ok());
    assert!(f("b").is_ok());
    assert!(f("c").is_ok());
    assert!(f("d").is_ok());
}

#[test]
fn many_segments() {
    fn f() -> std::io::Result<bool> {
        let db = try!(lsm::db::new(tempfile("many_segments"), lsm::DEFAULT_SETTINGS));

        const NUM : usize = 5000;
        const EACH : usize = 10;

        let mut a = Vec::new();
        for i in 0 .. NUM {
            let g = try!(db.WriteSegmentFromSortedSequence(lsm::GenerateNumbers {cur: i * EACH, end: (i+1) * EACH, step: 1}));
            a.push(g);
        }
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(a.clone()));
        }

        let res : std::io::Result<bool> = Ok(true);
        res
    }
    assert!(f().is_ok());
}

#[test]
fn one_blob() {
    fn f() -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("one_blob"), lsm::DEFAULT_SETTINGS));

        const LEN : usize = 100000;

        let mut v = Vec::new();
        for i in 0 .. LEN {
            v.push(i as u8);
        }
        assert_eq!(LEN, v.len());
        let mut t2 = std::collections::HashMap::new();
        insert_pair_string_blob(&mut t2, "e", lsm::Blob::Array(v.into_boxed_slice()));
        let g2 = try!(db.WriteSegment2(t2));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g2]));
        }

        let mut csr = try!(db.OpenCursor());
        assert_eq!(1, try!(count_keys_forward(&mut csr)));
        assert_eq!(1, try!(count_keys_backward(&mut csr)));

        try!(csr.First());
        assert!(csr.IsValid());
        assert_eq!(LEN, csr.ValueLength().unwrap().unwrap());
        let mut q = csr.Value().unwrap();

        match q {
            lsm::Blob::Tombstone => assert!(false),
            lsm::Blob::Array(ref a) => assert_eq!(LEN, a.len()),
            lsm::Blob::Stream(ref mut r) => {
                let mut a = Vec::new();
                try!(r.read_to_end(&mut a));
                assert_eq!(LEN, a.len());
            },
        }

        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn no_le_ge() {
    fn f() -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("no_le_ge"), lsm::DEFAULT_SETTINGS));
        let mut t1 = std::collections::HashMap::new();
        insert_pair_string_string(&mut t1, "c", "3");
        insert_pair_string_string(&mut t1, "g", "7");
        insert_pair_string_string(&mut t1, "e", "5");
        let g1 = try!(db.WriteSegment(t1));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g1]));
        }
        let mut csr = try!(db.OpenCursor());
        try!(csr.Seek(&to_utf8("a"), lsm::SeekOp::SEEK_LE));
        assert!(!csr.IsValid());

        try!(csr.Seek(&to_utf8("d"), lsm::SeekOp::SEEK_LE));
        assert!(csr.IsValid());

        try!(csr.Seek(&to_utf8("f"), lsm::SeekOp::SEEK_GE));
        assert!(csr.IsValid());

        try!(csr.Seek(&to_utf8("h"), lsm::SeekOp::SEEK_GE));
        assert!(!csr.IsValid());

        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn seek_ge_le_bigger() {
    fn f() -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("seek_ge_le_bigger"), lsm::DEFAULT_SETTINGS));
        let mut t1 = std::collections::HashMap::new();
        for i in 0 .. 10000 {
            let sk = format!("{}", i*2);
            let sv = format!("{}", i);
            insert_pair_string_string(&mut t1, &sk, &sv);
        }
        let g = try!(db.WriteSegment(t1));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g]));
        }
        let mut csr = try!(db.OpenCursor());
        try!(csr.Seek(&to_utf8("8088"), lsm::SeekOp::SEEK_EQ));
        assert!(csr.IsValid());

        try!(csr.Seek(&to_utf8("8087"), lsm::SeekOp::SEEK_EQ));
        assert!(!csr.IsValid());

        try!(csr.Seek(&to_utf8("8087"), lsm::SeekOp::SEEK_LE));
        assert!(csr.IsValid());
        assert_eq!("8086", from_utf8(csr.Key().unwrap()));

        try!(csr.Seek(&to_utf8("8087"), lsm::SeekOp::SEEK_GE));
        assert!(csr.IsValid());
        assert_eq!("8088", from_utf8(csr.Key().unwrap()));

        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn seek_ge_le() {
    fn f() -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("seek_ge_le"), lsm::DEFAULT_SETTINGS));
        let mut t1 = std::collections::HashMap::new();
        insert_pair_string_string(&mut t1, "a", "1");
        insert_pair_string_string(&mut t1, "c", "3");
        insert_pair_string_string(&mut t1, "e", "5");
        insert_pair_string_string(&mut t1, "g", "7");
        insert_pair_string_string(&mut t1, "i", "9");
        insert_pair_string_string(&mut t1, "k", "11");
        insert_pair_string_string(&mut t1, "m", "13");
        insert_pair_string_string(&mut t1, "o", "15");
        insert_pair_string_string(&mut t1, "q", "17");
        insert_pair_string_string(&mut t1, "s", "19");
        insert_pair_string_string(&mut t1, "u", "21");
        insert_pair_string_string(&mut t1, "w", "23");
        insert_pair_string_string(&mut t1, "y", "25");
        let g = try!(db.WriteSegment(t1));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g]));
        }
        let mut csr = try!(db.OpenCursor());
        assert_eq!(13, try!(count_keys_forward(&mut csr)));
        assert_eq!(13, try!(count_keys_backward(&mut csr)));

        try!(csr.Seek(&to_utf8("n"), lsm::SeekOp::SEEK_EQ));
        assert!(!csr.IsValid());

        try!(csr.Seek(&to_utf8("n"), lsm::SeekOp::SEEK_LE));
        assert!(csr.IsValid());
        assert_eq!("m", from_utf8(csr.Key().unwrap()));

        try!(csr.Seek(&to_utf8("n"), lsm::SeekOp::SEEK_GE));
        assert!(csr.IsValid());
        assert_eq!("o", from_utf8(csr.Key().unwrap()));

        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn tombstone() {
    fn f() -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("tombstone"), lsm::DEFAULT_SETTINGS));
        let mut t1 = std::collections::HashMap::new();
        insert_pair_string_string(&mut t1, "a", "1");
        insert_pair_string_string(&mut t1, "b", "2");
        insert_pair_string_string(&mut t1, "c", "3");
        insert_pair_string_string(&mut t1, "d", "4");
        let g1 = try!(db.WriteSegment(t1));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g1]));
        }
        let mut t2 = std::collections::HashMap::new();
        insert_pair_string_blob(&mut t2, "b", lsm::Blob::Tombstone);
        let g2 = try!(db.WriteSegment2(t2));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g2]));
        }
        // TODO it would be nice to check the multicursor without the living wrapper
        let mut csr = try!(db.OpenCursor());
        try!(csr.First());
        assert!(csr.IsValid());
        assert_eq!("a", from_utf8(csr.Key().unwrap()));
        assert_eq!("1", from_utf8(ReadValue(csr.Value().unwrap()).unwrap()));

        try!(csr.Next());
        assert!(csr.IsValid());
        assert_eq!("c", from_utf8(csr.Key().unwrap()));
        assert_eq!("3", from_utf8(ReadValue(csr.Value().unwrap()).unwrap()));

        try!(csr.Next());
        assert!(csr.IsValid());
        assert_eq!("d", from_utf8(csr.Key().unwrap()));
        assert_eq!("4", from_utf8(ReadValue(csr.Value().unwrap()).unwrap()));

        try!(csr.Next());
        assert!(!csr.IsValid());

        assert_eq!(3, try!(count_keys_forward(&mut csr)));
        assert_eq!(3, try!(count_keys_backward(&mut csr)));

        try!(csr.Seek(&to_utf8("b"), lsm::SeekOp::SEEK_EQ));
        assert!(!csr.IsValid());

        try!(csr.Seek(&to_utf8("b"), lsm::SeekOp::SEEK_LE));
        assert!(csr.IsValid());
        assert_eq!("a", from_utf8(csr.Key().unwrap()));
        try!(csr.Next());
        assert!(csr.IsValid());
        assert_eq!("c", from_utf8(csr.Key().unwrap()));

        try!(csr.Seek(&to_utf8("b"), lsm::SeekOp::SEEK_GE));
        assert!(csr.IsValid());
        assert_eq!("c", from_utf8(csr.Key().unwrap()));
        try!(csr.Prev());
        assert_eq!("a", from_utf8(csr.Key().unwrap()));

        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn overwrite() {
    fn f() -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("overwrite"), lsm::DEFAULT_SETTINGS));
        let mut t1 = std::collections::HashMap::new();
        insert_pair_string_string(&mut t1, "a", "1");
        insert_pair_string_string(&mut t1, "b", "2");
        insert_pair_string_string(&mut t1, "c", "3");
        insert_pair_string_string(&mut t1, "d", "4");
        let g1 = try!(db.WriteSegment(t1));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g1]));
        }
        fn getb(db: &lsm::db) -> std::io::Result<String> {
            let mut csr = try!(db.OpenCursor());
            try!(csr.Seek(&to_utf8("b"), lsm::SeekOp::SEEK_EQ));
            Ok(from_utf8(ReadValue(csr.Value().unwrap()).unwrap()))
        }
        assert_eq!("2", getb(&db).unwrap());
        let mut t2 = std::collections::HashMap::new();
        insert_pair_string_string(&mut t2, "b", "5");
        let g2 = try!(db.WriteSegment(t2));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g2]));
        }
        assert_eq!("5", getb(&db).unwrap());

        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn blobs_of_many_sizes() {
    fn f() -> std::io::Result<()> {
        let settings = lsm::DbSettings {
                DefaultPageSize : 256,
                PagesPerBlock : 4,
                .. lsm::DEFAULT_SETTINGS
            };
        let db = try!(lsm::db::new(tempfile("blobs_of_many_sizes"), settings));
        // TODO why doesn't Box<[u8]> support clone?
        // for now, we have a function to generate the pile we need, and we call it twice
        fn gen() -> std::collections::HashMap<Box<[u8]>,Box<[u8]>> {
            let mut t1 = std::collections::HashMap::new();
            for i in 200 .. 1500 {
                let k = format!("{}", i);
                let mut v = String::new();
                for j in 0 .. i {
                    let s = format!("{}", j);
                    v.push_str(&s);
                }
                insert_pair_string_string(&mut t1, &k, &v);
            }
            t1
        }
        let g1 = try!(db.WriteSegment(gen()));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g1]));
        }
        let mut csr = try!(db.OpenCursor());
        let t1 = gen(); // generate another copy
        for (k,v) in t1 {
            try!(csr.Seek(&k, lsm::SeekOp::SEEK_EQ));
            assert!(csr.IsValid());
            assert_eq!(v.len(), csr.ValueLength().unwrap().unwrap());
            assert_eq!(v, ReadValue(csr.Value().unwrap()).unwrap());
        }
        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn write_then_read() {
    fn f() -> std::io::Result<()> {
        fn write(name: &str) -> std::io::Result<()> {
            let db = try!(lsm::db::new(String::from_str(name), lsm::DEFAULT_SETTINGS));
            let mut d = std::collections::HashMap::new();
            for i in 1 .. 100 {
                let s = format!("{}", i);
                insert_pair_string_string(&mut d, &s, &s);
            }
            let g = try!(db.WriteSegment(d));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g]));
        }
            let mut d = std::collections::HashMap::new();
            insert_pair_string_blob(&mut d, "73", lsm::Blob::Tombstone);
            let g = try!(db.WriteSegment2(d));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g]));
        }
            Ok(())
        }

        fn read(name: &str) -> std::io::Result<()> {
            let db = try!(lsm::db::new(String::from_str(name), lsm::DEFAULT_SETTINGS));
            let mut csr = try!(db.OpenCursor());
            try!(csr.Seek(&format!("{}", 42).into_bytes().into_boxed_slice(), lsm::SeekOp::SEEK_EQ));
            assert!(csr.IsValid());
            try!(csr.Next());
            assert_eq!("43", from_utf8(csr.Key().unwrap()));
            try!(csr.Seek(&format!("{}", 73).into_bytes().into_boxed_slice(), lsm::SeekOp::SEEK_EQ));
            assert!(!csr.IsValid());
            try!(csr.Seek(&format!("{}", 73).into_bytes().into_boxed_slice(), lsm::SeekOp::SEEK_LE));
            assert!(csr.IsValid());
            assert_eq!("72", from_utf8(csr.Key().unwrap()));
            try!(csr.Next());
            assert!(csr.IsValid());
            assert_eq!("74", from_utf8(csr.Key().unwrap()));
            Ok(())
        }

        let name = tempfile("write_then_read");
        try!(write(&name));
        try!(read(&name));
        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn prefix_compression() {
    fn f() -> std::io::Result<()> {
        let db = try!(lsm::db::new(tempfile("prefix_compression"), lsm::DEFAULT_SETTINGS));
        let mut d = std::collections::HashMap::new();
        for i in 1 .. 10000 {
            let s = format!("{}", i);
            insert_pair_string_string(&mut d, &("prefix_compression".to_string() + &s), &s);
        }
        let g = try!(db.WriteSegment(d));
        {
            let lck = try!(db.GetWriteLock());
            try!(lck.commitSegments(vec![g]));
        }
        let mut csr = try!(db.OpenCursor());
        try!(csr.First());
        assert!(csr.IsValid());
        Ok(())
    }
    assert!(f().is_ok());
}

#[test]
fn threads() {
    fn f() -> std::io::Result<()> {
        use std::sync::Arc;
        use std::thread;

        let settings = lsm::DbSettings {
                DefaultPageSize : 256,
                PagesPerBlock : 4,
                .. lsm::DEFAULT_SETTINGS
            };
        let db = try!(lsm::db::new(tempfile("threads"), settings));
        let data = Arc::new(db);

        let h1 = {
            let data = data.clone();
            let h = thread::spawn(move || -> std::io::Result<()> {
                let g = try!(data.WriteSegmentFromSortedSequence(lsm::GenerateNumbers {cur: 0, end: 10000, step: 1}));
                Ok(())
            });
            h
        };

        let h2 = {
            let data = data.clone();
            let h = thread::spawn(move || -> std::io::Result<()> {
                let g = try!(data.WriteSegmentFromSortedSequence(lsm::GenerateNumbers {cur: 20000, end: 30000, step: 1}));
                Ok(())
            });
            h
        };

        h1.join();
        h2.join();

        Ok(())
    }

    assert!(f().is_ok());
}

