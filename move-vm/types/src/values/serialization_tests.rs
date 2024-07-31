// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Contains tests for serialization
//!
use crate::{
    delayed_values::delayed_field_id::DelayedFieldID,
    value_serde::{serialize_and_allow_delayed_values, serialized_size_allowing_delayed_values},
    values::{Struct as StructValue, Value},
};
use claims::{assert_err, assert_ok, assert_some};
use move_core_types::{
    account_address::AccountAddress,
    u256,
    value::{IdentifierMappingKind, MoveStruct, MoveStructLayout, MoveTypeLayout, MoveValue},
};

#[test]
fn enum_round_trip() {
    let layout = MoveTypeLayout::Struct(MoveStructLayout::RuntimeVariants(vec![
        vec![MoveTypeLayout::U64],
        vec![],
        vec![MoveTypeLayout::Bool, MoveTypeLayout::U32],
    ]));
    let good_values = vec![
        MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![MoveValue::U64(42)])),
        MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![])),
        MoveValue::Struct(MoveStruct::RuntimeVariant(2, vec![
            MoveValue::Bool(true),
            MoveValue::U32(13),
        ])),
    ];
    for value in good_values {
        let blob = value.simple_serialize().expect("serialization succeeds");
        let de_value =
            MoveValue::simple_deserialize(&blob, &layout).expect("deserialization succeeds");
        assert_eq!(value, de_value, "roundtrip serialization succeeds")
    }
    let bad_tag_value = MoveValue::Struct(MoveStruct::RuntimeVariant(3, vec![MoveValue::U64(42)]));
    let blob = bad_tag_value
        .simple_serialize()
        .expect("serialization succeeds");
    MoveValue::simple_deserialize(&blob, &layout)
        .inspect_err(|e| {
            assert!(
                e.to_string().contains("invalid value"),
                "unexpected error message: {}",
                e
            );
        })
        .expect_err("bad tag value deserialization fails");
    let bad_struct_value = MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::U64(42)]));
    let blob = bad_struct_value
        .simple_serialize()
        .expect("serialization succeeds");
    MoveValue::simple_deserialize(&blob, &layout)
        .inspect_err(|e| {
            assert!(
                e.to_string().contains("end of input"),
                "unexpected error message: {}",
                e
            );
        })
        .expect_err("bad struct value deserialization fails");
}

#[test]
fn test_serialized_size() {
    use IdentifierMappingKind::*;
    use MoveStructLayout::*;
    use MoveTypeLayout::*;

    let u64_delayed_value = Value::delayed_value(DelayedFieldID::new_with_width(12, 8));
    let u128_delayed_value = Value::delayed_value(DelayedFieldID::new_with_width(123, 16));
    let derived_string_delayed_value = Value::delayed_value(DelayedFieldID::new_with_width(12, 60));

    // First field is a string, second field is a padding to ensure constant size.
    let derived_string_layout = Struct(Runtime(vec![
        Struct(Runtime(vec![Vector(Box::new(U8))])),
        Vector(Box::new(U8)),
    ]));

    // All these pairs should serialize.
    let good_values_layouts_sizes = [
        (Value::u8(10), U8),
        (Value::u16(10), U16),
        (Value::u32(10), U32),
        (Value::u64(10), U64),
        (Value::u128(10), U128),
        (Value::u256(u256::U256::one()), U256),
        (Value::bool(true), Bool),
        (Value::address(AccountAddress::ONE), Address),
        (Value::signer(AccountAddress::ONE), Signer),
        (u64_delayed_value, Native(Aggregator, Box::new(U64))),
        (u128_delayed_value, Native(Snapshot, Box::new(U128))),
        (
            derived_string_delayed_value,
            Native(DerivedString, Box::new(derived_string_layout)),
        ),
        (
            Value::vector_address(vec![AccountAddress::ONE]),
            Vector(Box::new(Address)),
        ),
        (
            Value::struct_(StructValue::pack(vec![
                Value::bool(true),
                Value::vector_u32(vec![1, 2, 3, 4, 5]),
            ])),
            Struct(Runtime(vec![Bool, Vector(Box::new(U32))])),
        ),
    ];
    for (value, layout) in good_values_layouts_sizes {
        let bytes = assert_some!(assert_ok!(serialize_and_allow_delayed_values(
            &value, &layout
        )));
        let size = assert_ok!(serialized_size_allowing_delayed_values(&value, &layout));
        assert_eq!(size, bytes.len());
    }

    // Also test unhappy path, mostly mismatches in value-layout.
    let u64_delayed_value = Value::delayed_value(DelayedFieldID::new_with_width(0, 8));
    let malformed_delayed_value = Value::delayed_value(DelayedFieldID::new_with_width(1, 7));
    let bad_values_layouts_sizes = [
        (Value::u8(10), U16),
        (u64_delayed_value, U64),
        (malformed_delayed_value, U64),
        (Value::u64(12), Native(Aggregator, Box::new(U64))),
    ];
    for (value, layout) in bad_values_layouts_sizes {
        assert_err!(serialized_size_allowing_delayed_values(&value, &layout));
    }
}
