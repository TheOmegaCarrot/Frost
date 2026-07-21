//! Slicing, grouping, sorting, searching, and transforming arrays and maps.

use std::collections::BTreeMap;

use crate::{FrostError, FrostMap, FrostType, Param, Params, Value};

pub(super) fn keys_global() -> Value {
    super::stub("keys")
}

pub(super) fn values_global() -> Value {
    super::stub("values")
}

pub(super) fn map_keys_global() -> Value {
    super::stub("map_keys")
}

pub(super) fn map_values_global() -> Value {
    super::stub("map_values")
}

pub(super) fn len_global() -> Value {
    super::stub("len")
}

pub(super) fn range_global() -> Value {
    super::stub("range")
}

pub(super) fn nulls_global() -> Value {
    super::stub("nulls")
}

pub(super) fn repeat_global() -> Value {
    super::stub("repeat")
}

pub(super) fn id_global() -> Value {
    super::stub("id")
}

pub(super) fn has_global() -> Value {
    super::stub("has")
}

pub(super) fn includes_global() -> Value {
    super::stub("includes")
}

pub(super) fn index_global() -> Value {
    super::stub("index")
}

pub(super) fn dig_global() -> Value {
    super::stub("dig")
}

pub(super) fn slice_global() -> Value {
    super::stub("slice")
}

pub(super) fn stride_global() -> Value {
    super::stub("stride")
}

pub(super) fn take_global() -> Value {
    super::stub("take")
}

pub(super) fn drop_global() -> Value {
    super::stub("drop")
}

pub(super) fn tail_global() -> Value {
    super::stub("tail")
}

pub(super) fn drop_tail_global() -> Value {
    super::stub("drop_tail")
}

pub(super) fn slide_global() -> Value {
    super::stub("slide")
}

pub(super) fn chunk_global() -> Value {
    super::stub("chunk")
}

pub(super) fn reverse_global() -> Value {
    super::stub("reverse")
}

pub(super) fn take_while_global() -> Value {
    super::stub("take_while")
}

pub(super) fn drop_while_global() -> Value {
    super::stub("drop_while")
}

pub(super) fn chunk_by_global() -> Value {
    super::stub("chunk_by")
}

pub(super) fn flatten_global() -> Value {
    super::stub("flatten")
}

pub(super) fn zip_global() -> Value {
    super::stub("zip")
}

pub(super) fn zip_with_global() -> Value {
    super::stub("zip_with")
}

pub(super) fn xprod_global() -> Value {
    super::stub("xprod")
}

pub(super) fn xprod_with_global() -> Value {
    super::stub("xprod_with")
}

pub(super) fn transform_global() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::STRUCTURED),
        Param::of(FrostType::FUNCTION),
    ]);

    Value::checked_native("transform", PARAMS, |mut ctx, args| {
        let structure = args[0].take();
        let function = args[1].take();

        match structure {
            Value::Array(arr) => {
                let mut vec = arr.into_vec();
                for elem in vec.iter_mut() {
                    *elem = ctx.invoke(&function, [elem.take()])?;
                }
                Ok(vec.into())
            }
            Value::Map(map) => {
                let mut map = map.into_map();
                let mut result: Value = FrostMap::empty().into();
                for (k, v) in map.into_iter() {
                    let invoke_result = ctx.invoke(&function, [k.into(), v])?;
                    result = Value::add_owned(
                        result,
                        match invoke_result {
                            Value::Map(_) => invoke_result,
                            _ => {
                                return Err(FrostError::from_string(format!(
                                    "When transforming a Map, the function must return a Map, got {}",
                                    invoke_result.type_name()
                                )));
                            }
                        },
                    )?;
                }
                Ok(result)
            }
            _ => unreachable!(),
        }
    })
}

pub(super) fn flat_map_global() -> Value {
    super::stub("flat_map")
}

pub(super) fn select_global() -> Value {
    super::stub("select")
}

pub(super) fn reject_global() -> Value {
    super::stub("reject")
}

pub(super) fn fold_global() -> Value {
    super::stub("fold")
}

pub(super) fn sum_global() -> Value {
    super::stub("sum")
}

pub(super) fn product_global() -> Value {
    super::stub("product")
}

pub(super) fn sorted_global() -> Value {
    super::stub("sorted")
}

pub(super) fn sort_by_global() -> Value {
    super::stub("sort_by")
}

pub(super) fn any_global() -> Value {
    super::stub("any")
}

pub(super) fn all_global() -> Value {
    super::stub("all")
}

pub(super) fn none_global() -> Value {
    super::stub("none")
}

pub(super) fn find_global() -> Value {
    super::stub("find")
}

pub(super) fn group_by_global() -> Value {
    super::stub("group_by")
}

pub(super) fn count_by_global() -> Value {
    super::stub("count_by")
}

pub(super) fn scan_global() -> Value {
    super::stub("scan")
}

pub(super) fn partition_global() -> Value {
    super::stub("partition")
}

pub(super) fn map_into_global() -> Value {
    super::stub("map_into")
}

pub(super) fn to_entries_global() -> Value {
    super::stub("to_entries")
}

pub(super) fn from_entries_global() -> Value {
    super::stub("from_entries")
}

pub(super) fn dissoc_global() -> Value {
    super::stub("dissoc")
}

pub(super) fn each_global() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::STRUCTURED),
        Param::of(FrostType::FUNCTION),
    ]);
    Value::checked_native("each", PARAMS, |mut ctx, params| {
        let structure = params[0].take();
        let function = params[1].take();
        match &structure {
            Value::Array(arr) => {
                for v in arr.iter() {
                    ctx.invoke_ref(&function, [v])?;
                }
            }
            Value::Map(map) => {
                for (k, v) in map.iter() {
                    ctx.invoke(&function, [k.clone().into(), v.clone()])?;
                }
            }
            _ => unreachable!(),
        };

        Ok(structure)
    })
}
