#!/usr/bin/env sh

VERSION=$(grep -oP '"vercel-rust":\s*"\K[^"]*' package.json)
echo "matched version: $VERSION"
sed -i -E "s/(\"runtime\":\s*\"vercel-rust@)[^\"]*/\1$VERSION/" vercel.json