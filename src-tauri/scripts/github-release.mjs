// Downloads one asset of a GitHub release, for the fetch-*.mjs scripts.
//
// The release-download link can answer 504 for hours (8 Nov: every release of several repos,
// while the API's asset endpoint served the same files), so each attempt tries the link, then
// the API. 5xx and dropped connections are retried with a growing wait; any other 4xx is
// final. GITHUB_TOKEN, when set, lifts the API's limit of 60 unauthenticated calls an hour.
// Callers check the bytes against a pinned hash.

function apiHeaders(accept) {
  const headers = { Accept: accept, "User-Agent": "aeroedits-fetch" };
  if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
  return headers;
}

async function tryFetch(url, headers) {
  try {
    const response = await fetch(url, { headers });
    if (response.ok) return { response };
    return { status: response.status, failure: `Download failed (${response.status}): ${url}` };
  } catch (error) {
    return { failure: `Download failed (${error.cause?.code ?? error.message}): ${url}` };
  }
}

async function apiAssetUrl(repo, tag, asset) {
  const url = `https://api.github.com/repos/${repo}/releases/tags/${tag}`;
  const result = await tryFetch(url, apiHeaders("application/vnd.github+json"));
  if (!result.response) return { ...result, failure: `Asset lookup: ${result.failure}` };
  const release = await result.response.json();
  const found = release.assets?.find((a) => a.name === asset);
  if (!found) return { status: 404, failure: `Asset lookup: ${url} has no asset ${asset}` };
  return { url: found.url };
}

const retryable = (status) => status === undefined || status >= 500 || status === 429;

export async function downloadReleaseAsset(repo, tag, asset, attempts = 5) {
  const link = `https://github.com/${repo}/releases/download/${tag}/${asset}`;
  let apiUrl;
  for (let attempt = 1; ; attempt++) {
    const direct = await tryFetch(link);
    if (direct.response) return Buffer.from(await direct.response.arrayBuffer());
    console.log(`${direct.failure}; trying the GitHub API`);
    let result = direct;
    if (!apiUrl) {
      const lookup = await apiAssetUrl(repo, tag, asset);
      if (lookup.url) apiUrl = lookup.url;
      else result = lookup;
    }
    if (apiUrl) {
      result = await tryFetch(apiUrl, apiHeaders("application/octet-stream"));
      if (result.response) return Buffer.from(await result.response.arrayBuffer());
    }
    if (attempt >= attempts || !(retryable(direct.status) && retryable(result.status))) {
      throw new Error(result.failure);
    }
    const wait = 5 * attempt;
    console.log(`${result.failure}; retrying in ${wait} s (attempt ${attempt + 1} of ${attempts})`);
    await new Promise((resolve) => setTimeout(resolve, wait * 1000));
  }
}
