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

        tabular: Vec<Tabular>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Tabular {
        first: u8,
        second: u8,
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

                //TODO: Empty tabular sections should still create empty lists, and make sure the list didn't exist before.
                # [[tabular].{first, second}]
                0, 1
                2, 3
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

            tabular: vec![
                Tabular {
                    first: 0,
                    second: 1,
                },
                Tabular {
                    first: 2,
                    second: 3,
                },
            ]
        })
    );
}
