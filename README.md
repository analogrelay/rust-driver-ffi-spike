# Prototypes for Cosmos Rust Driver interop

## Overview

This repo contains a simplified "mock" Cosmos Driver which has a very simplistic query pipeline.
The purpose is to sketch out the FFI interfaces between Rust and Cosmos, and to provide a
harness for benchmarking some potential optimizations

## Perforamnce Hypotheses

Here are some of the performance hypotheses we are exploring, and the current results.

When testing, we generate a JSON object with a deep nested object, and then run a query pipeline.

The "depth" of the payload is shown as `X by Y`, which means we create `X` levels of arrays, each with `Y` items (which in turn have `X - 1` levels below them).

For example, this is a `2 by 2` object:

```json
{
    "id": 42,
    "title": "hello",
    "huge_data": [
        {
            "key": [ 
                { "key": "value" }, 
                { "key": "value" } 
            ]
        },
        {
            "key": [ 
                { "key": "value" }, 
                { "key": "value" } 
            ]
        }
    ]
}

### Using `serde_json::value::RawValue` can avoid parsing large user payloads

The pipeline has to parse JSON from the consumer, use that JSON to run the query pipeline, and then return a serialized version back to the consumer to be deserialized by the language-specific code.
This is a lot of JSON parsing and serialization, but we may be able to use `serde_json::value::RawValue`, which keeps a section of the JSON _unparsed_ to avoid some deserialization.

This hypothesis can be tested by running `script/run --raw-value [language]`, which will run the benchmark with `raw_value` enabled in the driver.

Example run **without** `raw_value` (JSON object depth `6 by 10`):

```
> ./script/run go
Building rust driver in go mode...
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
Running Go sample...
Go: Creating pipeline
Go: Enqueued data for partition p1 in 6.02233434s
Go: Enqueued data for partition p2 in 6.713611222s
Go: Iterating pipeline
Go: pipeline_next_item_raw took 824.059053ms
Go: Received Item Partition 1 / Item 0 / Order 1
Go: pipeline_next_item_raw took 878.003016ms
Go: Received Item Partition 1 / Item 1 / Order 2
Go: pipeline_next_item_raw took 876.644539ms
Go: Received Item Partition 2 / Item 0 / Order 2
Go: pipeline_next_item_raw took 815.812345ms
Go: Received Item Partition 1 / Item 2 / Order 3
Go: pipeline_next_item_raw took 1.011416888s
Go: Received Item Partition 1 / Item 3 / Order 2
Go: pipeline_next_item_raw took 1.198557128s
Go: Received Item Partition 1 / Item 4 / Order 3
Go: pipeline_next_item_raw took 876.38365ms
Go: Received Item Partition 2 / Item 1 / Order 3
Go: pipeline_next_item_raw took 1.041713558s
Go: Received Item Partition 2 / Item 2 / Order 2
Go: pipeline_next_item_raw took 863.742159ms
Go: Received Item Partition 2 / Item 3 / Order 3
Go: pipeline_next_item_raw took 853.207629ms
Go: Received Item Partition 2 / Item 4 / Order 2
Go: pipeline_next_item_raw took 7.949µs
Go: Freeing pipeline
```

Example run **with** `raw_value` (JSON object depth `6 by 10`):

```
 ./script/run go --raw-value
Building rust driver in go mode...
   Compiling cosmos_driver v0.1.0 (/home/ashleyst/code/analogrelay/rust-driver-ffi-spike/cosmos_driver)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.41s
Running Go sample...
Go: Creating pipeline
Go: Enqueued data for partition p1 in 1.062979066s
Go: Enqueued data for partition p2 in 1.206158673s
Go: Iterating pipeline
Go: pipeline_next_item_raw took 7.435049ms
Go: Received Item Partition 1 / Item 0 / Order 1
Go: pipeline_next_item_raw took 4.163777ms
Go: Received Item Partition 1 / Item 1 / Order 2
Go: pipeline_next_item_raw took 1.513864ms
Go: Received Item Partition 2 / Item 0 / Order 2
Go: pipeline_next_item_raw took 1.355376ms
Go: Received Item Partition 1 / Item 2 / Order 3
Go: pipeline_next_item_raw took 4.518631ms
Go: Received Item Partition 1 / Item 3 / Order 2
Go: pipeline_next_item_raw took 1.666661ms
Go: Received Item Partition 1 / Item 4 / Order 3
Go: pipeline_next_item_raw took 1.338658ms
Go: Received Item Partition 2 / Item 1 / Order 3
Go: pipeline_next_item_raw took 1.505705ms
Go: Received Item Partition 2 / Item 2 / Order 2
Go: pipeline_next_item_raw took 1.41582ms
Go: Received Item Partition 2 / Item 3 / Order 3
Go: pipeline_next_item_raw took 1.949307ms
Go: Received Item Partition 2 / Item 4 / Order 2
Go: pipeline_next_item_raw took 9.519µs
Go: Freeing pipeline
```

Notice there is a significant improvement in both the insertion time and the iteration time.
The insertion time ("Enqueued data for partition") is reduced because the driver does not have to parse the JSON payload, while it does still have to copy the bytes.
Similarly, the iteration time is also reduced because the driver can take those "raw" JSON bytes and "loan" them to the caller, which can deserialize them without having to copy them.