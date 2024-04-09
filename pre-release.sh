#!/usr/bin/env bash

set -e

NEW_VERSION=$1
if [[ -z "${NEW_VERSION}" ]]; then
  echo "usage: $0 <new version>"
  exit 1
fi

if [[ -z "${CHANGELOG_GITHUB_TOKEN}" ]]; then
  echo "You need to set a CHANGELOG_GITHUB_TOKEN environment variable with a Github token"
  echo "Click this link to generate the token, it doesn't need any additional scopes"
  echo "    https://github.com/settings/tokens/new?description=Dinghy%20Changelog%20Generator"
  exit 1
fi

if ! gem list -i github_changelog_generator > /dev/null; then
  echo "Please install the ruby gem github_changelog_generator using the following command:"
  echo "    gem install github_changelog_generator"
  exit 1
fi

if ! git diff-index --quiet HEAD --; then
  echo "Git workspace is not clean, clean it before proceeding"
  exit 1
fi

echo "Generating changelog"

PATH="$(gem environment user_gemhome)/bin:$(gem environment gemhome)/bin"  github_changelog_generator --no-verbose -u sonos -p dinghy --include-tags-regex "^0\..+\..+" --future-release "${NEW_VERSION}"

printf "Here are the updates to the changelog \n\n\n"

git --no-pager diff

printf "\n\n\nDoes this seem correct (Y/n)? "

read -r ANSWER

if [[ -n "${ANSWER}" ]] && ! [[ "${ANSWER}" == [Yy]* ]]; then
  echo "Aborting, go fix things"
  exit 1
fi

git commit -am "changelog for $NEW_VERSION"

