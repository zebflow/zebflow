# Files

Project files live in ZebFS. The first backend uses the local project `files/`
directory, while the public model is object based so other storage backends can
be added.

## FileRef

A FileRef is a small JSON value that points to a file. It can describe an upload,
a ZebFS object, or a temporary file without copying the file bytes into every
node payload.

```json
{
  "__zf_type": "file_ref",
  "backend": "zebfs",
  "ref": "uploads/roads.geojson",
  "path": "uploads/roads.geojson",
  "url": "/fs/owner/project/uploads/roads.geojson",
  "size": 24810,
  "sha256": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  "filename": "roads.geojson",
  "mime": "application/geo+json",
  "kind": "geojson",
  "lifecycle": "durable"
}
```

The exact fields are defined in the [format contract](../contracts/formats.md).

## Common Work

The file nodes can list, inspect, read, write, delete, copy, move, and create
folders. Other nodes can compress, extract, make thumbnails, convert PDF files,
and save validated uploads.

## Uploads

A browser or webhook upload enters as a FileRef. Nodes that accept files should
resolve the reference and read the bytes. They must not write the FileRef JSON
itself as the file content.

Multiple uploaded files are represented as a list of FileRef values. Select one
by its list position or process the list with a bounded loop.

## Access

Files are private by default. Public access is an explicit ZebFS rule. Making a
folder public applies to objects under that folder according to the ACL rules.

The normal file response is an object download. Rendering active HTML should be
an explicit web server decision on a controlled site origin, not the default
file behavior.

## Large Data

Keep large bytes in storage. Pass a FileRef between nodes. Use table, geo, map,
or media nodes to process the referenced object.
