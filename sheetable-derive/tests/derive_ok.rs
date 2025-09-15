use serde_json::json;
use sheetable::{Sheetable};

#[derive(sheetable_derive::SheetableReadOnly, Debug, Clone)]
struct UserDetails {
    #[column("C")]
    elo: u64,
}

#[derive(sheetable_derive::Sheetable, Debug, Clone)]
struct User<RO> {
    #[column("A")]
    id: u64,
    #[column("B")]
    name: String,
    #[calculated(UserDetails)]
    details: RO,
}

#[test]
fn user_hydrated_roundtrip() {
    let base = User { id: 1, name: "Mario".into(), details: () };
    assert_eq!(base.to_values().unwrap(), vec![json!(1), json!("Mario")]);

    let hydrated: User<UserDetails> =
        <User<()> as Sheetable>::from_values(&[json!(1), json!("Mario"), json!(1500)]).unwrap();
    assert_eq!(hydrated.details.elo, 1500);

    assert_eq!(hydrated.to_values().unwrap(), vec![json!(1), json!("Mario")]);
}

#[derive(sheetable_derive::Sheetable, Debug, Clone)]
struct Player {
    #[column("A")]
    id: u64,
    #[column("B")]
    name: String,
}

#[test]
fn player_roundtrip() {
    let p = Player { id: 7, name: "Rosalina".into() };
    let cells = p.to_values().unwrap();
    assert_eq!(cells, vec![json!(7), json!("Rosalina")]);
    let back: Player = Player::from_values(&cells).unwrap();
    assert_eq!(back.name, "Rosalina");
}
