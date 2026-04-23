# How Marketplace Works

Every Zebflow instance can act as a marketplace source.

## Core ideas

- one Zebflow can publish packs
- another Zebflow can consume them
- repositories can be local or remote
- public packs can be browsed without token
- private access can use scoped tokens

## Why this matters

This lets Zebflow share project material natively, without forcing everything through GitHub repos or plugin packaging first.

## Common flow

1. publish pack from one Zebflow instance
2. add that marketplace base URL as a repository in another instance
3. browse packs in Marketplace or `Add+`
4. add the pack into the local project workspace
