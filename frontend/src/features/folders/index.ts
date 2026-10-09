/**
 * The folders feature (ORG-05): the sidebar folder tree with drag and drop,
 * the folder, overview and Unfiled pages, "Add to folder" for meeting rows
 * and the meeting header, auto-add rules and the ⌘K folder results. Other
 * code imports from here only.
 */
export { foldersApi } from './api';
export { AddToFolderSubmenu, useNewFolderFor } from './components/add-to-folder';
export { FolderNavSection } from './components/folder-tree';
export { FolderScreen } from './components/folder-screen';
export { FoldersPalette } from './components/folders-palette';
export { MeetingFolders } from './components/meeting-folders';
export { FoldersScreen, UnfiledScreen } from './components/overview-screens';
export { foldersCopy } from './copy';
export { meetingDragSource } from './dnd';
export { folderKeys, folderQuery, folderTreeQuery, useFolderTree } from './queries';
export { asFolderId, type Folder, type FolderId, type FolderTreeEntry } from './types';
