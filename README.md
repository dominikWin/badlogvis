# badlogvis

`badlogvis` is a data log visualization tool that parses badlog bag files or CSV and outputs a single self-contained HTML file.

## Usage

```
badlogvis [FLAGS] <input> [output]
```

See `badlogvis --help` for flags.

Normal usage is `badlogvis example.bag`.
CSV can be parsed with `badlogvis -c example.csv`.

## Install

Can be run directly from binary release.

Can be installed by cloning repo and using cargo to install:

```
git clone https://github.com/dominikWin/badlogvis.git
cd badlogvis
cargo install
```

Binary is created with `cargo build --release` or `cargo build --release --target=x86_64-pc-windows-gnu` for Windows cross compile.

## Data Types

badlogvis supports has three types of data: Topics, Values, and Event Logs.

### Values

A value is a key-value pair of strings. (Key is usually called name)

### Topics

Topics are data point that changes over time.
Each topic has a name (String), unit (String), attributes (set of Strings), and data-points (Either String or Double-precision floating-point).

The units of a topic are shown in graphs of that topic and derived units are used for derived topics (So the derivative of "Amps" with an xaxis unit of "s" creates a new unit of "Amps/s").

### Event Logs

An event log is a standard topic with the `log` attribute. It is used for traditional event logging. These are an extension and are not native to the badlog format.

## Namespace

The name of both topics and values may consist of any letter, number or a `' '`, `'_'`, and `'/'`.

The foreword slash is used to denote a names folder. For example a topic named `Drivetrain/Wheels/A Position` has a base name of `A Position` and folder `Drivetrain/Wheels`.

Any names without a folder, like `Time`, are put into the root folder.

When presented all folders are sorted in alphabetical order (case insensitive) and all members retain their original order.

## Attributes

Each topic can have attributes assigned to it which change how badlogvis draws it.

The `hide` attribute prevents outputting a direct graph of the topic. An derivative topics are still output. If this is the only attribute the topic is not parsed at all. badlogvis does not hide data by any other attribute so it is usually used to suppress input data while still showing derived data.

The `log` attribute defines a topic as an event log. It must be the only attribute on that topic. Both an empty value and any data that can be parsed as numeric is discarded. Any data kept is timestamped and added to a standard text based event log.

The `area` attribute draws the output as an area graph instead of a line graph.

The `xaxis` attribute marks this topic as the x-axis for the rest of the data. This can only be applied to one topic.
If no topic has the `xaxis` attribute then the index is used as the x-axis, and "Index" is used in any derived units.

The `zero` attribute makes sure that its graph's y-axis starts at 0.

The `join:<topic>` attribute adds this topic as a series to a combined line graph. An example is `join:Drivetrain/Positions`. The `<topic>` must not be the name of an input topic.
To see any benefit from this at least two topics should be joined to the same graph.

There are several deriving attributes that insert a new graph with the processed data. No attributes can be set to these virtual topics.

| Attribute | Description |
| --------- | ----------- |
| `differentiate` | Add derivative graph |
| `integrate` | Add integral graph |
| `delta` | Add delta graph |

Note that if you don't set a `xaxis` topic then the derivative and delta will be the same.

## Changelog

#### 0.3.2:
Add automated TravisCI builds for Windows/OSX/Linux.

#### 0.3.1:
Allow for attaching files with the `-a`/`--attach` flag.

#### 0.3:
Add event logging support with `log` attribute.
