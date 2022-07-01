fn equidistant(seq: &[i32]) -> bool {
  if seq.len() <= 2 {
    return true;
  }
  let delta = seq[1] - seq[0];
  for window in seq.windows(2) {
    if let [a, b] = window {
      if b - a != delta {
        return false;
      }
    } else {
      unreachable!();
    }
  }
  true
}

pub fn extend(seq: &[i32]) -> (Vec<i32>, Vec<i32>) {
  assert!(!seq.is_empty());
  let last = *seq.last().unwrap();
  let mut kept = vec![last + 1];
  let mut removed = Vec::new();
  for &x in seq.iter().rev() {
    kept.push(x);
    if kept.len() >= 4 && equidistant(&kept[kept.len() - 4..]) {
      removed.push(
        kept.remove(kept.len() - 2)
      );
    }
  }
  kept.reverse();
  (kept, removed)
}

#[test]
fn test_extend() {
  let expected = [
    vec![0],
    vec![0, 1],
    vec![0, 1, 2],
    vec![0,    2, 3],
    vec![0,    2, 3, 4],
    vec![0,    2,    4, 5],
    vec![0,    2,    4, 5, 6],
    vec![0,          4,    6, 7],
    vec![0,          4,    6, 7, 8],
    vec![0,          4,    6,    8, 9],
    vec![0,          4,    6,    8, 9, 10],
    vec![0,          4,          8,    10, 11],
    vec![0,          4,          8,    10, 11, 12],
    vec![0,          4,          8,    10,     12, 13],
    vec![0,          4,          8,    10,     12, 13, 14],
    vec![0,                      8,            12,     14, 15],
  ];
  let mut current = vec![0];
  for seq in expected {
    assert_eq!(current, seq);
    current = extend(&current).0;
  }
}
