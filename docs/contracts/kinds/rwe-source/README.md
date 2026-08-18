# RweSource

Status: pending review

This contract defines one authored RWE source file: a page, a component, a `.ts`
behavior script, or a stylesheet. It is the content a project **adds** rather
than installs, so it becomes the receiving project's own editable source.

It has no envelope. A `.tsx` file is governed by convention — which imports are
legal, what a page must export, where files may live — rather than by a
serialization format, which is why it is the first kind with a `SourceFile`
representation.

Its review will cover the import rules, entry and export requirements, the
project-relative layout, what a distributable set of source files must carry
with it, and the collision behaviour when adding a file that already exists.

## Why it exists

Recorded during the distribution review. `template_bundle` distributes these
files today, `folder_bundle` distributes many of them, and the UI catalog adds
them from a built-in set — three paths, no contract governing the shape any of
them carries.

Matrix row 13 covers the RWE *protocol*: compile, render, event, and error
messages. Its own next action is to "separate source, compiled artifact, and
wire protocol contracts". This kind is the source half of that separation.
