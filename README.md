# Tokei Serverless API

This is a fork of the [Tokei badge service](https://github.com/XAMPPRocky/tokei_rs) that runs as a Vercel serverless function, utilizing Vercel's [edge caching](https://vercel.com/docs/concepts/functions/serverless-functions/edge-caching#) for fast responses. This API creates a badge that will display like ![Lines of Code](https://aschey.tech/tokei/github/aschey/vercel-tokei). You can use the service hosted at `https://aschey.tech/tokei` or you can fork this repo and host it on your personal Vercel account.

## Motivation

Hosting Tokei on a traditional server has the inherent issue of filling up disk space because Tokei works by cloning repositories. This can cause the service to go down if the disk space on the server fills up. Running on a serverless platform mitigates this issue because the container that runs the service is ephemeral. If someone requests a repository that crashes the service, it should still work fine for other users because it will just spin up a separate container.

## URL Scheme

```sh
https://aschey.tech/tokei/<domain>[.com]/<namespace>/<repository>[?category=<category>&format=<format>&style=<style>&labelColor=<labelColor>&color=<color>&label=<label>&cacheSeconds=<cacheSeconds>]
```

All querystring parameters are optional.

### Custom Options

- **category**: Which metric is displayed
  - **valid options**: `code`, `blanks`, `comments`, `files`
  - **default**: `code`
- **format**: Output format
  - **valid options**: `svg` or `json`
  - **default**: `svg`

### Standard Options used by [shields.io](https://shields.io/)

- **style**: SVG badge style
  - **valid options**: `flat`, `flat-square`, `plastic`, `social`, or `for-the-badge`
  - **default**: `flat`
- **label**: Override the default label text
  - **default**: Defaults to the label that matches the category
- **labelColor**: Background color of the label on the left side
  - **valid options**: `brightgreen`, `green`, `yellow`, `yellowgreen`, `orange`, `red`, `blue`, `grey`, `lightgrey`, or any valid CSS color. Note that CSS color strings like `hsl(195, 100%, 50%)` must be properly url encoded. You can omit the leading `#` from hex colors.
  - **default**: `grey`
- **color**: Background color of the metric on the right side
  - **valid options**: same as above
  - **default**: `blue`
- **cacheSeconds**: How long to cache the response for. We use Vercel's [`stale-while-revalidate`](https://vercel.com/docs/concepts/functions/serverless-functions/edge-caching#stale-while-revalidate) option to maximize cache efficiency
  - **valid options**: Any number >= 60
  - **default**: `60`

## Self Hosting

To host this API yourself, you can fork this repository and connect your fork to your Vercel account. Once deployed, your API should be available at `your-subdomain.vercel.app/tokei`.

## Running Locally

Install the [Vercel CLI](https://vercel.com/docs/cli). Once installed, run `cargo build` in the `api` directory and then run `vercel dev` from the root directory. The site should be available at `localhost:3000/tokei`. If you're on Windows, you may need to run this with Git Bash or WSL. There seems to be an issue with running the Rust serverless runtime on PowerShell.
