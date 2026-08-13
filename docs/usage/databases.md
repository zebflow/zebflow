# Databases

Zebflow gives each project an embedded Sekejap database and can connect to
external databases such as PostgreSQL and SQLite.

## Choose the Data Store

Use Sekejap when you want project local records, graph links, text search,
spatial values, or vectors without another database server.

Use PostgreSQL when the data already lives there, several services share it, or
you need PostgreSQL features and operations.

Use SQLite for small local relational data and portable database files.

## Connections and Credentials

A connection says where and how a database is reached. A credential keeps the
secret values. Keep these separate so a connection can be inspected without
showing its password or token.

## Pipeline Nodes

Current database work includes:

- `n.sekejap.query` for Sekejap SQL reads and general SQL
- `n.sekejap.insert` for fast structured inserts, including large batches
- `n.pg.query` for PostgreSQL
- `n.sqlite.query` for SQLite reads
- `n.sqlite.mutate` for SQLite writes

Read each live node definition before authoring. The definition states its
fields, input, output, examples, and errors.

## Safe Data Work

- Use bound values instead of joining user input into SQL text.
- Inspect the schema before writing a query.
- Sample a few rows before making assumptions about stored values.
- Use a transaction when several changes must succeed together.
- Use bulk operations for large writes.
- Keep long maintenance work away from request paths.

## Sekejap Maintenance

Sekejap uses durable files and a write ahead log. Compact it through Zebflow's
database management surface when needed. Do not open the same embedded store
from two independent processes at the same time. Back up a consistent store,
not a random set of files copied during a write.
