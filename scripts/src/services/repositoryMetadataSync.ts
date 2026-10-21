export interface RepositoryMetadata {
  name: string;
  description: string;
  tags: string[];
  license?: string;
}

interface GithubRepository {
  name: string;
  description: string | null;
  topics?: string[];
  license?: { spdx_id?: string | null; name?: string | null } | null;
}

interface GitlabRepository {
  name: string;
  description: string | null;
  topics?: string[];
  license?: {
    spdx_id?: string | null;
    name?: string | null;
    nickname?: string | null;
  } | null;
}

async function fetchJson<T>(url: string, headers?: HeadersInit): Promise<T> {
  const response = await fetch(url, { headers });
  if (!response.ok) {
    throw new Error(`Repository provider returned ${response.status} ${response.statusText}.`);
  }
  return response.json() as Promise<T>;
}

function normalizeMetadata(
  name: string,
  description: string | null,
  topics: string[] | undefined,
  license?: string | null
): RepositoryMetadata {
  if (!name.trim()) throw new Error('Repository response did not include a project name.');
  return {
    name: name.trim(),
    description: description?.trim() ?? '',
    tags: [...new Set((topics ?? []).map((topic) => topic.trim()).filter(Boolean))],
    license: license?.trim() || undefined,
  };
}

export async function fetchRepositoryMetadata(repositoryUrl: string): Promise<RepositoryMetadata> {
  let url: URL;
  try {
    url = new URL(repositoryUrl);
  } catch {
    throw new Error('Enter a valid GitHub or GitLab repository URL.');
  }

  if (url.protocol !== 'https:') {
    throw new Error('Repository URL must use HTTPS.');
  }
  if (url.username || url.password || url.port) {
    throw new Error('Repository URL must not include credentials or a custom port.');
  }

  const path = url.pathname.replace(/^\/+|\/+$/g, '').replace(/\.git$/i, '');
  const segments = path.split('/').filter(Boolean);

  if (url.hostname === 'github.com' || url.hostname === 'www.github.com') {
    if (segments.length !== 2) throw new Error('GitHub URL must point to an owner and repository.');
    const repository = await fetchJson<GithubRepository>(
      `https://api.github.com/repos/${encodeURIComponent(segments[0])}/${encodeURIComponent(segments[1])}`,
      { Accept: 'application/vnd.github+json', 'X-GitHub-Api-Version': '2022-11-28' }
    );
    const license = repository.license?.spdx_id;
    return normalizeMetadata(
      repository.name,
      repository.description,
      repository.topics,
      license && license !== 'NOASSERTION' ? license : repository.license?.name
    );
  }

  if (url.hostname === 'gitlab.com' || url.hostname === 'www.gitlab.com') {
    if (segments.length < 2) throw new Error('GitLab URL must point to a namespace and repository.');
    const projectPath = encodeURIComponent(segments.join('/'));
    const [repository, license] = await Promise.all([
      fetchJson<GitlabRepository>(`https://gitlab.com/api/v4/projects/${projectPath}`),
      fetchJson<GitlabRepository['license']>(`https://gitlab.com/api/v4/projects/${projectPath}/license`)
        .catch(() => null),
    ]);
    const licenseId = license?.spdx_id ?? repository.license?.spdx_id;
    return normalizeMetadata(
      repository.name,
      repository.description,
      repository.topics,
      licenseId && licenseId !== 'NOASSERTION'
        ? licenseId
        : license?.name ?? license?.nickname ?? repository.license?.name
    );
  }

  throw new Error('Only public repositories on github.com and gitlab.com are supported.');
}