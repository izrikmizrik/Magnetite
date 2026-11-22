package dev.magnetite;

import android.database.Cursor;
import android.database.MatrixCursor;
import android.os.CancellationSignal;
import android.os.ParcelFileDescriptor;
import android.provider.DocumentsContract.Document;
import android.provider.DocumentsContract.Root;
import android.provider.DocumentsProvider;
import android.webkit.MimeTypeMap;

import java.io.File;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.Comparator;
import java.util.Locale;

/**
 * The game folder (getFilesDir()/game) as a Storage Access Framework provider, so the system's
 * Files app can browse the saves, options.txt and logs, and copy files in and out.
 *
 * <p>A document id is the path relative to the root folder, with '/' separators; the root itself
 * is "". Every id becomes a canonical file and is refused unless it is the root or under it, so
 * nothing else in the app's storage can be reached.
 */
public class MagnetiteDocumentsProvider extends DocumentsProvider {
    private static final String ROOT_ID = "magnetite";
    private static final String ROOT_TITLE = "Magnetite";

    private static final String[] DEFAULT_ROOT_PROJECTION = {
            Root.COLUMN_ROOT_ID,
            Root.COLUMN_TITLE,
            Root.COLUMN_DOCUMENT_ID,
            Root.COLUMN_MIME_TYPES,
            Root.COLUMN_FLAGS,
            Root.COLUMN_ICON,
            Root.COLUMN_AVAILABLE_BYTES,
    };

    private static final String[] DEFAULT_DOCUMENT_PROJECTION = {
            Document.COLUMN_DOCUMENT_ID,
            Document.COLUMN_MIME_TYPE,
            Document.COLUMN_DISPLAY_NAME,
            Document.COLUMN_LAST_MODIFIED,
            Document.COLUMN_FLAGS,
            Document.COLUMN_SIZE,
    };

    @Override
    public boolean onCreate() {
        return true;
    }

    /** The provider's root folder: the player's game folder, made on first use. */
    private File root() {
        File folder = new File(getContext().getFilesDir(), "game");
        if (!folder.exists()) {
            folder.mkdirs();
        }
        return folder;
    }

    /** The canonical form of `file`, so ".." and links can be judged by where they land. */
    private static File canonical(File file) throws FileNotFoundException {
        try {
            return file.getCanonicalFile();
        } catch (IOException error) {
            throw new FileNotFoundException("Unreadable " + file);
        }
    }

    /** `file`, refused unless it is the root or under it. */
    private static File inside(File root, File file) throws FileNotFoundException {
        File resolved = canonical(file);
        if (!resolved.toPath().startsWith(canonical(root).toPath())) {
            throw new FileNotFoundException("Outside the game folder: " + file);
        }
        return resolved;
    }

    /** The file a document id names. */
    private File fileFor(String documentId) throws FileNotFoundException {
        File folder = root();
        return inside(folder, documentId.isEmpty() ? folder : new File(folder, documentId));
    }

    /** The inverse of {@link #fileFor}: the path relative to the root, with '/' separators. */
    private String documentIdFor(File file) throws FileNotFoundException {
        Path rootPath = canonical(root()).toPath();
        Path path = inside(root(), file).toPath();
        return rootPath.relativize(path).toString().replace(File.separatorChar, '/');
    }

    private static String mimeTypeOf(File file) {
        if (file.isDirectory()) {
            return Document.MIME_TYPE_DIR;
        }
        String name = file.getName();
        int dot = name.lastIndexOf('.');
        if (dot >= 0) {
            String extension = name.substring(dot + 1).toLowerCase(Locale.ROOT);
            String type = MimeTypeMap.getSingleton().getMimeTypeFromExtension(extension);
            if (type != null) {
                return type;
            }
        }
        return "application/octet-stream";
    }

    private static int flagsOf(File file) {
        int flags = Document.FLAG_SUPPORTS_DELETE | Document.FLAG_SUPPORTS_RENAME;
        if (file.isDirectory()) {
            return flags | Document.FLAG_DIR_SUPPORTS_CREATE;
        }
        return flags | Document.FLAG_SUPPORTS_WRITE;
    }

    /** One document row, filled by column name so any projection comes out in its own order. */
    private void include(MatrixCursor cursor, File file) throws FileNotFoundException {
        String id = documentIdFor(file);
        String[] columns = cursor.getColumnNames();
        Object[] values = new Object[columns.length];
        for (int i = 0; i < columns.length; i++) {
            switch (columns[i]) {
                case Document.COLUMN_DOCUMENT_ID:
                    values[i] = id;
                    break;
                case Document.COLUMN_MIME_TYPE:
                    values[i] = mimeTypeOf(file);
                    break;
                case Document.COLUMN_DISPLAY_NAME:
                    values[i] = id.isEmpty() ? ROOT_TITLE : file.getName();
                    break;
                case Document.COLUMN_LAST_MODIFIED:
                    values[i] = file.lastModified();
                    break;
                case Document.COLUMN_FLAGS:
                    values[i] = flagsOf(file);
                    break;
                case Document.COLUMN_SIZE:
                    values[i] = file.isDirectory() ? 0 : file.length();
                    break;
                default:
                    values[i] = null;
                    break;
            }
        }
        cursor.addRow(values);
    }

    @Override
    public Cursor queryRoots(String[] projection) {
        MatrixCursor cursor = new MatrixCursor(
                projection != null ? projection : DEFAULT_ROOT_PROJECTION);
        File root = root();
        String[] columns = cursor.getColumnNames();
        Object[] values = new Object[columns.length];
        for (int i = 0; i < columns.length; i++) {
            switch (columns[i]) {
                case Root.COLUMN_ROOT_ID:
                    values[i] = ROOT_ID;
                    break;
                case Root.COLUMN_TITLE:
                    values[i] = ROOT_TITLE;
                    break;
                case Root.COLUMN_DOCUMENT_ID:
                    values[i] = "";
                    break;
                case Root.COLUMN_MIME_TYPES:
                    values[i] = "*/*";
                    break;
                case Root.COLUMN_FLAGS:
                    values[i] = Root.FLAG_SUPPORTS_CREATE
                            | Root.FLAG_SUPPORTS_IS_CHILD
                            | Root.FLAG_LOCAL_ONLY;
                    break;
                case Root.COLUMN_ICON:
                    values[i] = getContext().getApplicationInfo().icon;
                    break;
                case Root.COLUMN_AVAILABLE_BYTES:
                    values[i] = root.getFreeSpace();
                    break;
                default:
                    values[i] = null;
                    break;
            }
        }
        cursor.addRow(values);
        return cursor;
    }

    @Override
    public Cursor queryDocument(String documentId, String[] projection)
            throws FileNotFoundException {
        MatrixCursor cursor = new MatrixCursor(
                projection != null ? projection : DEFAULT_DOCUMENT_PROJECTION);
        include(cursor, fileFor(documentId));
        return cursor;
    }

    @Override
    public Cursor queryChildDocuments(String parentDocumentId, String[] projection,
            String sortOrder) throws FileNotFoundException {
        File parent = fileFor(parentDocumentId);
        File[] children = parent.listFiles();
        if (children == null) {
            throw new FileNotFoundException("Not a folder: " + parentDocumentId);
        }
        Arrays.sort(children, Comparator.comparing(File::getName));
        MatrixCursor cursor = new MatrixCursor(
                projection != null ? projection : DEFAULT_DOCUMENT_PROJECTION);
        for (File child : children) {
            try {
                include(cursor, child);
            } catch (FileNotFoundException ignored) {
                // A link leading out of the folder is not ours to show; the rest still list.
            }
        }
        return cursor;
    }

    @Override
    public ParcelFileDescriptor openDocument(String documentId, String mode,
            CancellationSignal signal) throws FileNotFoundException {
        return ParcelFileDescriptor.open(fileFor(documentId), ParcelFileDescriptor.parseMode(mode));
    }

    @Override
    public String createDocument(String parentDocumentId, String mimeType, String displayName)
            throws FileNotFoundException {
        File parent = fileFor(parentDocumentId);
        File file = inside(root(), uniqueChild(parent, plainName(displayName)));
        boolean made;
        try {
            made = Document.MIME_TYPE_DIR.equals(mimeType) ? file.mkdir() : file.createNewFile();
        } catch (IOException error) {
            throw new FileNotFoundException("Cannot create " + file);
        }
        if (!made) {
            throw new FileNotFoundException("Cannot create " + file);
        }
        return documentIdFor(file);
    }

    /** A name for one entry of a folder, never a path. */
    private static String plainName(String name) throws FileNotFoundException {
        if (name == null || name.isEmpty() || name.indexOf('/') >= 0 || name.indexOf('\\') >= 0) {
            throw new FileNotFoundException("Not a file name: " + name);
        }
        return name;
    }

    /** `displayName`, or "name (1).ext", "name (2).ext"... when the folder already holds it. */
    private static File uniqueChild(File parent, String displayName) {
        File file = new File(parent, displayName);
        int dot = displayName.lastIndexOf('.');
        String stem = dot > 0 ? displayName.substring(0, dot) : displayName;
        String suffix = dot > 0 ? displayName.substring(dot) : "";
        for (int n = 1; file.exists(); n++) {
            file = new File(parent, stem + " (" + n + ")" + suffix);
        }
        return file;
    }

    @Override
    public void deleteDocument(String documentId) throws FileNotFoundException {
        // The root is the game folder itself; only what is inside it may go.
        if (documentId.isEmpty()) {
            throw new FileNotFoundException("Not deleting the game folder itself");
        }
        deleteTree(fileFor(documentId));
    }

    private static void deleteTree(File file) throws FileNotFoundException {
        // A link is removed itself, never followed, so nothing outside the folder is touched.
        if (!Files.isSymbolicLink(file.toPath())) {
            File[] children = file.listFiles();
            if (children != null) {
                for (File child : children) {
                    deleteTree(child);
                }
            }
        }
        if (!file.delete() && file.exists()) {
            throw new FileNotFoundException("Cannot delete " + file);
        }
    }

    @Override
    public String renameDocument(String documentId, String displayName)
            throws FileNotFoundException {
        File file = fileFor(documentId);
        File target = inside(root(), new File(file.getParentFile(), plainName(displayName)));
        if (!file.renameTo(target)) {
            throw new FileNotFoundException("Cannot rename " + file);
        }
        return documentIdFor(target);
    }

    @Override
    public boolean isChildDocument(String parentDocumentId, String documentId) {
        try {
            return fileFor(documentId).toPath().startsWith(fileFor(parentDocumentId).toPath());
        } catch (FileNotFoundException error) {
            return false;
        }
    }

    @Override
    public String getDocumentType(String documentId) throws FileNotFoundException {
        return mimeTypeOf(fileFor(documentId));
    }
}
