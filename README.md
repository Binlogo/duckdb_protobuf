# duckdb_protobuf

a duckdb extension for parsing sequences of protobuf messages encoded in either 
the standard varint delimited format or a u32 big endian delimited format.

## quick start

ensure you're using duckdb 0.10.1

```
$ duckdb -version
v0.10.1 4a89d97db8
```

currently only works with duckdb 0.10.1 due to breaking changes in the C ABI
in versions following. waiting for the rust bindings to merge support for 1.0 to
upgrade.

build the extension from source using `cargo build --release`. make sure to 
use release mode as the performance delta between release and debug mode is
often greater than an order of magnitude.

an attempt was made to provide prebuilt binaries but it seems macOS code-signing
and the dynamic linker have issues with loading duckdb extensions built in other
environments. will revisit this in the future.

start duckdb with `-unsigned` flag to allow loading unsigned libraries

```bash
$ duckdb -unsigned
```

or if you're using the jdbc connector, you can do this with the
`allow_unsigned_extensions` jdbc connection property.

next load the extension

```sql
LOAD '/Users/martin/projects/duckdb_protobuf/target/release/libduckdb_protobuf.dylib';
```

now start shredding up your protobufs!

```sql
SELECT *
FROM protobuf(
    descriptors = './descriptor.pb',
    files = './scrape/data/SceneVersion/**/*.bin',
    message_type = 'test_server.v1.GetUserSceneVersionResponse',
    delimiter = 'BigEndianFixed'
)
LIMIT 10;
```

## why

sometimes you want to land your row primary data in a format with a well-defined
structure and pretty good decode performance and poke around without a load
step. maybe you're scraping an endpoint which returns protobuf responses, you're
figuring out the schema as you go and iteration speed matters much more than 
query performance.

`duckdb_protobuf` allows for making a new choice along the
flexibility-performance tradeoff continuum for fast exploration of protobuf 
streams with little upfront load complexity or time.

## configuration

* `descriptors`: path to the protobuf descriptor file. Generated using something
  like `protoc --descriptor_set_out=descriptor.pb ...`
* `files`: glob pattern for the files to read. Uses the [`glob`][glob] crate 
  for evaluating globs.
* `message_type`: the fully qualified message type to parse.

## features

* converts `google.protobuf.Timestamp` messages to duckdb timestamp
* supports nested messages with repeating fields

## limitations

* doesn't support a few types (bytes, maps, {s,}fixed{32,64}, sint{32,64}), 
  contributions and even feedback that these field types are used is welcome!
* execution is single threaded (limitations of the rust bindings)

i'm releasing this to understand how other folks are using protobuf streams and
duckdb. i'm open to PRs, issues and other feedback.

[glob]: https://docs.rs/glob/latest/glob/