import type { InstalledSkill } from './types';

export function findInstalledGithubSkill(
  installed: InstalledSkill[],
  githubUrl: string,
  folderName: string
): InstalledSkill | undefined {
  let repository: URL;
  try {
    repository = new URL(githubUrl);
  } catch {
    return undefined;
  }
  const [owner, repo] = repository.pathname.toLowerCase().split('/').filter(Boolean);
  if (repository.hostname !== 'github.com' || !owner || !repo) return undefined;

  const name = folderName.replaceAll(' ', '-').toLowerCase();
  return installed.find((skill) => {
    const installedName = skill.name.toLowerCase();
    if (installedName !== name && installedName !== `${owner}-${name}`) return false;
    if (!skill.source_repo) return true;
    try {
      const source = new URL(skill.source_repo.includes('://')
        ? skill.source_repo : `https://github.com/${skill.source_repo}`);
      const [sourceOwner, sourceRepo] = source.pathname.toLowerCase().split('/').filter(Boolean);
      return source.hostname === 'github.com' && sourceOwner === owner
        && sourceRepo?.replace(/\.git$/, '') === repo.replace(/\.git$/, '');
    } catch {
      return false;
    }
  });
}
