# Hub Package Format

Hub packages are shareable source packages.

Current package families include:

- template packages
- pipeline packages
- folder packages
- library packages
- example packages
- project bundles
- node bundles

Node bundles can contain one node or many nodes. A single node is still represented as a bundle with one node definition, so install, review, indexing, and security scanning can use one path.

Hub packages should be inspectable before add. The review step should show files, package kind, metadata, external URLs, credential needs, initialization steps, and other policy warnings.
