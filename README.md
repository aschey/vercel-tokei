# Tokei Serverless API

This is a fork of the [Tokei badge service](https://github.com/XAMPPRocky/tokei_rs) that runs as a Vercel serverless function. This API creates a badge that will display like ![Lines of Code](https://aschey.tech/tokei/github/aschey/vercel-tokei). You can use the service hosted at `https://aschey.tech/tokei` or you can fork this repo and host it on your personal Vercel account.

## URL Scheme

```
https://aschey.tech/tokei/<domain>[.com]/<namespace>/<repository>[?category=<category>&format=<format>&style=<style>&labelColor=<labelColor>&color=<color>&cacheSeconds=<cacheSeconds>]
```

All querystring parameters are optional:

- **category**: Which metric is displayed
  - **valid options**: `code`, `blanks`, `comments`, `files`
  - **default**: `code`
- **format**: Output format
  - **valid options**: `svg` or `json`
  - **default**: `svg`
- **style**: SVG badge style
  - **valid options**: `flat`, `flat-square`, `plastic`, `social`, or `for-the-badge`
  - **default**: `flat`
- **labelColor**: Background color of the label on the left side
  - **valid options**: `brightgreen`, `green`, `yellow`, `yellowgreen`, `orange`, `red`, `blue`, `grey`, `lightgrey`, or any valid CSS color. Note that CSS color strings like `hsl(195, 100%, 50%)` must be properly url encoded.
  - **default**: `grey`
- **color**: Background color of the metric on the right side
  - **valid options**: same as above
  - **default**: blue
- **cacheSeconds**: How long to cache the response for
  - **valid options**: Any number >= 60
  - **default**: 60

## Self Hosting

To host this API yourself, you can fork this repository and connect your fork to your Vercel account. Once deployed, your API should be available at `your-subdomain.vercel.app/tokei`.

## Running Locally

Install the [Vercel CLI](https://vercel.com/docs/cli). Once installed, run `vercel dev`. If you're on Windows, you may need to run this with Git Bash or WSL. There seems to be an issue with running the Rust serverless runtime on PowerShell.

## Limitations

The original API invokes the Git CLI in order to do a shallow clone of the repositories and count their statistics. However, installing and invoking external dependencies on a custom Vercel serverless runtime is a bit of a pain, so instead we use [libgit2](https://github.com/libgit2/libgit2) to clone repositories. Unfortunately libgit2 does not yet support shallow clones, so **this service will not handle large repositories with lengthy commit histories well**. Once shallow clone support lands in libgit2 or [gitoxide](https://github.com/Byron/gitoxide), this limitation should improve.
