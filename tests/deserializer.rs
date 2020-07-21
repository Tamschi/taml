use {serde::Deserialize, taml::deserializer::from_str};


//TODO: Split up this test.
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

        empty_table: Vec<()>,

        tabular: Vec<Tabular>,

        variants: Vec<Enum>,

        unit_variant: Enum,
        weird_variant: Enum,
        newtype_variant: Enum,
        tuple_variant: Enum,

        r#false: bool,
        r#true: bool,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    enum Enum {
        Structured { i32: i32, f64: f64 },
        Tuple(u8, u8),
        Newtype(u8),
        Unit,
        Weird(),
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

                #

                unit: ()
                seq: (0, 1, 2)

                zero_u8: 0
                one_u8: 1

                zero_i8: 0
                one_i8: 1
                minus_one_i8: -1

                # [[empty_table]]

                # [[tabular].{first, second}]
                0, 1
                2, 3

                # [[tabular].{{first, second}}]
                4, 5

                # [variants]:Structured
                i32: 12345
                f64: 6789.0

                # [[variants]:Tuple]
                (0, 1)

                # [[variants]:Newtype]
                (3)

                # [[variants]]
                Unit

                # [[variants]:Weird]
                ()

                #

                unit_variant: Unit
                weird_variant: Weird()
                newtype_variant: Newtype(4)
                tuple_variant: Tuple(5, 6)

                false: false
                true: true
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

            empty_table: vec![],

            tabular: vec![
                Tabular {
                    first: 0,
                    second: 1,
                },
                Tabular {
                    first: 2,
                    second: 3,
                },
                Tabular {
                    first: 4,
                    second: 5,
                },
            ],

            variants: vec![
                Enum::Structured {
                    i32: 12345,
                    f64: 6789.0
                },
                Enum::Tuple(0, 1),
                Enum::Newtype(3),
                Enum::Unit,
                Enum::Weird(),
            ],

            unit_variant: Enum::Unit,
            weird_variant: Enum::Weird(),
            newtype_variant: Enum::Newtype(4),
            tuple_variant: Enum::Tuple(5, 6),

            r#false: false,
            r#true: true,
        })
    );
}
