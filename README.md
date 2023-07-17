# Tokei Serverless API

![license](https://img.shields.io/badge/License-MIT%20or%20Apache%202-green.svg)
[![CI](https://github.com/aschey/vercel-tokei/actions/workflows/build.yml/badge.svg)](https://github.com/aschey/vercel-tokei/actions/workflows/build.yml)
![GitHub repo size](https://img.shields.io/github/repo-size/aschey/vercel-tokei)

This is a fork of the [Tokei badge service](https://github.com/XAMPPRocky/tokei_rs) that runs as a Vercel serverless function, utilizing Vercel's [edge caching](https://vercel.com/docs/concepts/functions/serverless-functions/edge-caching#) for fast responses. You can use the service hosted at `https://aschey.tech/tokei` or you can fork this repo and host it on your personal Vercel account.

## Motivation

Hosting Tokei on a traditional server has the inherent issue of filling up disk space because Tokei works by cloning repositories. This can cause the service to go down if the disk space on the server fills up. Running on a serverless platform mitigates this issue because the container that runs the service is ephemeral. If someone requests a repository that crashes the service, it should still work fine for other users because it will just spin up a separate container.

## Examples

**Default (lines of code):** ![Lines of Code](https://aschey.tech/tokei/github/aschey/vercel-tokei)

**Blanks:** ![Blanks](https://aschey.tech/tokei/github/aschey/vercel-tokei?category=blanks)

**Comments:** ![Comments](https://aschey.tech/tokei/github/aschey/vercel-tokei?category=comments)

**Files:** ![Files](https://aschey.tech/tokei/github/aschey/vercel-tokei?category=files)

**Plastic**: ![Flat](https://aschey.tech/tokei/github/aschey/vercel-tokei?style=plastic)

**Flat Square**: ![Flat](https://aschey.tech/tokei/github/aschey/vercel-tokei?style=flat-square)

**Social**: ![Flat](https://aschey.tech/tokei/github/aschey/vercel-tokei?style=social)

**For the Badge**: ![Flat](https://aschey.tech/tokei/github/aschey/vercel-tokei?style=for-the-badge)

**Styled:** ![Styled](https://aschey.tech/tokei/github/aschey/vercel-tokei?labelColor=badbe6&color=32a852&style=for-the-badge&label=Lines&logo=https://simpleicons.org/icons/rust.svg)

**Logo Only:** ![Logo Only](https://aschey.tech/tokei/github/aschey/vercel-tokei?color=157c8c&style=for-the-badge&logo=https://simpleicons.org/icons/rust.svg&label=)

**Logo as Label:** ![Logo as Label](https://aschey.tech/tokei/github/aschey/vercel-tokei?color=9b73eb&style=for-the-badge&logo=https://simpleicons.org/icons/rust.svg&label=&logoAsLabel=true&labelColor=dbd3ed)

## URL Scheme

```sh
https://aschey.tech/tokei/<domain>[.com]/<namespace>/<repository>[?category=<category>&format=<format>&style=<style>&labelColor=<labelColor>&color=<color>&label=<label>&logo=<logo>&logoAsLabel=<logoAsLabel>&cacheSeconds=<cacheSeconds>]
```

All querystring parameters are optional.

### Standard Options used by [shields.io](https://shields.io/)

- **style**: SVG badge style

   - **valid options**: `flat`, `flat-square`, `plastic`, `social`, or `for-the-badge`
   - **default**: `flat`

- **label**: Override the default label text. Pass in an empty value (`label=`) to disable.

   - **default**: Defaults to the label that matches the category

- **labelColor**: Background color of the label on the left side

   - **valid options**: `brightgreen`, `green`, `yellow`, `yellowgreen`, `orange`, `red`, `blue`, `grey`, `lightgrey`, or any valid CSS color. Note that CSS color strings like `hsl(195, 100%, 50%)` must be properly url encoded. You can omit the leading `#` from hex colors.
   - **default**: `grey`

- **color**: Background color of the metric on the right side

   - **valid options**: same as above
   - **default**: `blue`

- **logo**: Logo that will appear before the label.

   - **valid options**: Value can be supplied in either of the following formats:

      - HTTP URL to a hosted svg icon. **Example:** `logo=https://www.svgrepo.com/show/513821/code.svg`.
      - Data URL containing a base64-encoded SVG.
         - **Note:** make sure you use URL-safe base64 encoding (`+` characters need to be encoded as `%2B`). Many tools do not default to this.
         - **Example:** `logo=data:image/svg%2Bxml;base64,PHN2ZyB4bWxucz0iaH...`.

   - **default**: None

- **cacheSeconds**: How long to cache the response for. We use Vercel's [`stale-while-revalidate`](https://vercel.com/docs/concepts/functions/serverless-functions/edge-caching#stale-while-revalidate) option to maximize cache efficiency

   - **valid options**: Any number >= 60
   - **default**: `60`

### Custom Options

- **category**: Which metric is displayed

   - **valid options**: `code`, `blanks`, `comments`, `files`
   - **default**: `code`

- **format**: Output format

   - **valid options**: `svg` or `json`
   - **default**: `svg`

- **logoAsLabel**: This setting only applies when a logo is supplied and the label is empty. If this setting is true, then the logo will use the label background color. If it is false, it will use the message background color.

   - **valid options**: `1` or `true` will be parsed as a truthy value. Everything else will be considered `false`.

## Self Hosting

To host this API yourself, you can fork this repository and connect your fork to your Vercel account. Once deployed, your API should be available at `your-subdomain.vercel.app/tokei`.

## Running Locally

Install the [Vercel CLI](https://vercel.com/docs/cli). Once installed, run `vercel dev` from the root directory. The site should be available at `localhost:3000/tokei/[domain]/[user]/[repo]`.
