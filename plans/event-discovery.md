# Event Discovery: What We Know and What's Next

## Background

We've been improving the `ws_events` example for viewing live events. Now we're looking at the other angle: discovering what events exist and what their schema looks like, *before* subscribing.

## The Event Data Model

An event is fundamentally:
- A **key-value map** where values can be boolean (0|1), integer, double, or string
- Each key-value pair has an **attribute**: `source`, `key`, or `data`
- Each key-value pair can have a **namespace** (though a key only exists in one namespace)
- Events are organized by **topic** — a hierarchical path with segments (topic0, topic1, ...)

## Discovery API: GetEventInstances

The SOAP-based Event Service at `/vapix/services` has a `GetEventInstances` operation that returns the full event catalog as an XML topic tree. The response contains `aev:MessageInstance` elements with rich schema info:

```xml
<aev:MessageInstance aev:isProperty="true">
  <aev:SourceInstance>
    <aev:SimpleItemInstance Name="port" Type="xsd:int">
      <aev:Value>1</aev:Value>
      <aev:Value>2</aev:Value>
    </aev:SimpleItemInstance>
  </aev:SourceInstance>
  <aev:DataInstance>
    <aev:SimpleItemInstance Name="active" Type="xsd:boolean"
                           isPropertyState="true" />
  </aev:DataInstance>
</aev:MessageInstance>
```

Each `SimpleItemInstance` declares:
- `Name` — the key name
- `Type` — `xsd:boolean`, `xsd:int`, `xsd:string`, etc.
- `Value` (optional, repeated) — enumerated possible values for source fields
- `isPropertyState` — marks the boolean state variable in data fields
- `NiceName` — human-readable label
- `isDeprecated` — marks deprecated events

## What's Already Implemented

**`crates/vapix/src/services/event1.rs`** has a working `GetEventInstances` request, but the response parser only extracts topic paths — it discards all schema information (source/key/data declarations, types, possible values, attributes).

Current `MessageInstance` struct:
```rust
pub struct MessageInstance {
    pub topic: Vec<String>,  // just the path segments
}
```

## What's Missing

To fully support event discovery, the parser needs to also extract:
1. **Source fields** — name, type, and enumerated values
2. **Key fields** — name and type  
3. **Data fields** — name, type, and `isPropertyState`
4. **Event attributes** — `isProperty`, `NiceName`, `isDeprecated`

## Plan

### Step 1: Enrich the `MessageInstance` type

Add fields to capture the schema declared in each `aev:MessageInstance`:

```rust
pub struct SimpleItemDeclaration {
    pub name: String,
    pub value_type: String,          // e.g. "xsd:boolean", "xsd:int"
    pub values: Vec<String>,         // enumerated possible values (if any)
}

pub struct MessageInstance {
    pub topic: Vec<String>,
    pub is_property: bool,
    pub source: Vec<SimpleItemDeclaration>,
    pub key: Vec<SimpleItemDeclaration>,
    pub data: Vec<SimpleItemDeclaration>,
}
```

### Step 2: Update the XML parser

Extend the `quick_xml::Reader`-based parser in `SoapResponse for EventInstances` to:
- When entering `aev:MessageInstance`, read its `aev:isProperty` attribute
- Track whether we're inside `aev:SourceInstance`, `aev:KeyInstance`, or `aev:DataInstance`
- When entering `aev:SimpleItemInstance`, read `Name` and `Type` attributes
- Collect `aev:Value` text children as enumerated values

### Step 3: Create an example to display the catalog

Add a new example (or extend an existing one) that calls `get_event_instances()` and prints the discovered events with their schemas in a readable format.

### Files to modify

- `crates/vapix/src/services/event1.rs` — enrich types and parser
- `crates/vapix/examples/` — new example for event discovery display

### Verification

- `cargo check -p rs4a-vapix` compiles
- Smoke test `event_1_get_event_instances_returns_ok` still passes
- New example compiles and (when run against a device) shows event schemas

## Future improvements

- Add `--json` flag to examples for NDJSON output, enabling jq-based filtering and formatting
