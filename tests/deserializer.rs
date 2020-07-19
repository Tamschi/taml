use {serde::Deserialize, taml::deserializer::from_str};

#[test]
fn deserializer() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Deserializable {
        #[serde(default)]
        none: Option<()>,
        #[serde(default)]
        some: Option<()>,

        unit: (),

        seq: Vec<u8>,

        zero_u8: u8,
        one_u8: u8,

        zero_i8: i8,
        one_i8: i8,
        minus_one_i8: i8,
    }

    assert_eq!(
        dbg!(from_str(
            "
                some: ()

                unit: ()
                seq: (0, 1, 2)

                zero_u8: 0
                one_u8: 1

                zero_i8: 0
                one_i8: 1
                minus_one_i8: -1
            ",
        )),
        Ok(Deserializable {
            none: None,
            some: Some(()),

            unit: (),

            seq: vec![0, 1, 2],

            zero_u8: 0,
            one_u8: 1,

            zero_i8: 0,
            one_i8: 1,
            minus_one_i8: -1,
        })
    );
}
