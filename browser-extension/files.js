export function parts(path) {
  const values = path.replaceAll('\\', '/').split('/').filter(Boolean);
  if (values.some(v => v === '.' || v === '..' || /[\x00-\x1f]/.test(v))) throw Error('Invalid folder path');
  return values;
}

export function fileEntries(files) {
  return [...files].map(file => ({ file, path: parts(file.webkitRelativePath || file.name).join('/'), directory: false }));
}

export async function folderEntries(handle, prefix = '') {
  const path = [...parts(prefix), handle.name].join('/');
  const result = [];
  for await (const child of handle.values()) {
    if (child.kind === 'directory') result.push(...await folderEntries(child, path));
    else result.push({ file: await child.getFile(), path: `${path}/${child.name}`, directory: false });
  }
  return result.length ? result : [{ file: null, path, directory: true }];
}

export async function droppedEntries(items) {
  // Capture handles synchronously: DataTransfer is protected after the drop handler returns.
  const pending = [...items].filter(item => item.kind === 'file').map(item =>
    item.getAsFileSystemHandle ? item.getAsFileSystemHandle() : Promise.resolve(null));
  const fallback = [...items].filter(item => item.kind === 'file').map(item => item.getAsFile()).filter(Boolean);
  const handles = await Promise.all(pending);
  if (!handles.some(Boolean)) return fileEntries(fallback);
  const result = [];
  for (const handle of handles.filter(Boolean)) {
    if (handle.kind === 'directory') result.push(...await folderEntries(handle));
    else result.push({ file: await handle.getFile(), path: handle.name, directory: false });
  }
  return result;
}
