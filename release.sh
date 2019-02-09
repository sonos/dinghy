#!/bin/sh

CRATE=$1
VERSION=$2
CRATES="dinghy-build dinghy-test dinghy-lib cargo-dinghy"

if [ -z "$VERSION" ]
then
    echo "Usage: $0 <crate> <version>" 
    echo crates order is: $CRATES
    exit 1
fi

set -ex

if [ "$CRATE" = "all" ]
then
    for c in $CRATES
    do
        $0 $c $VERSION
    done
    exit 0
fi

# set_version cargo-dinghy/Cargo.toml 0.3.0
set_version() {
    FILE=$1
    VERSION=$2
    sed -i.back "s/^version *= *\".*\"/version = \"$2\"/" $FILE
    sed -i.back "s/^\(dinghy-[^ =]*\).*/\\1 = \"$2\"/" $FILE
}

set_version $CRATE/Cargo.toml $VERSION
git commit . -m "release $CRATE/$VERSION"
(cd $CRATE ; cargo publish)

if [ "$CRATE" = "cargo-dinghy" ]
then
    git tag "v$VERSION"
fi
git tag "$CRATE/$VERSION"
git push --tags

