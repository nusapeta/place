use placeid::build::build_data;
use placeid::writer::write_index;
use std::path::Path;

#[test]
fn mini_fixture_has_expected_features() {
    let dir = tempfile::TempDir::new().unwrap();
    let (data, report) = build_data(Path::new("tests/fixtures/mini.pbf")).unwrap();
    let path = dir.path().join("mini.idx");
    write_index(&data, &path).unwrap();
    let index = placeid::Index::open(&path, true).unwrap();
    let mut names: Vec<_> = index
        .places()
        .iter()
        .map(|place| {
            (
                index.str_at(place.name_off, place.name_len),
                index.class(index.find_row(place.place_id).unwrap()),
                place.admin_id,
            )
        })
        .collect();
    names.sort();
    assert_eq!(data.places.len(), 13);
    assert_eq!(report.counters.features, 13);
    assert_eq!(report.counters.admins, 4);
    assert!(
        names
            .iter()
            .any(|(name, class, _)| *name == "Indomaret" && *class == "shop")
    );
    assert!(
        names
            .iter()
            .any(|(name, class, _)| *name == "Alfamart" && *class == "shop")
    );
    assert!(
        names
            .iter()
            .any(|(name, class, _)| *name == "Jalan Panjang" && *class == "highway")
    );
    assert!(
        names
            .iter()
            .any(|(name, class, _)| *name == "Pulau Contoh" && *class == "place")
    );
    assert!(
        names
            .iter()
            .any(|(name, class, _)| *name == "Gambir" && *class == "boundary")
    );
    assert!(
        names
            .iter()
            .any(|(name, _, admin_id)| *name == "Indomaret" && *admin_id > 0)
    );
    assert_eq!(
        names
            .iter()
            .filter(|(name, _, _)| *name == "Indomaret")
            .count(),
        2
    );
}
