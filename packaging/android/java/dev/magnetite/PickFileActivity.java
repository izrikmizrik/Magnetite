package dev.magnetite;

import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.os.Bundle;
import android.provider.OpenableColumns;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;

/**
 * The system's file picker (Storage Access Framework), for a mod (.hmod) or a touch button's
 * picture: a transparent activity that starts {@link Intent#ACTION_OPEN_DOCUMENT} and copies what
 * the player picked into {@code <cacheDir>/picked}, where the app may read it after the picker's
 * own grant has gone.
 *
 * <p>The copy is made on a thread of its own; the file appears only whole, because it is written
 * to {@code <name>.part} and renamed at the end. A copy that fails leaves {@code error.txt} with
 * the reason, written the same way. The Rust side ({@code crates/magnetite-android/src/pick.rs})
 * reads that folder, so the names and the two-file protocol here are its contract.
 *
 * <p>Started only from the game ({@code android:exported="false"}), with the MIME type in the
 * intent's "type" extra: images for a picture, any file for a mod, which has no type of its own.
 */
public class PickFileActivity extends Activity {
    /** The folder the copies land in, under the app's cache. */
    private static final String FOLDER = "picked";
    /** The name of the file a failed pick leaves, with the reason as its text. */
    private static final String ERROR = "error.txt";
    /** The most one pick may copy: a mod or a button's picture is far below it. */
    private static final long LIMIT_BYTES = 256L * 1024 * 1024;
    /** The longest name a copy takes, in characters. */
    private static final int NAME_LIMIT = 100;
    /** The name a pick with no usable display name takes. */
    private static final String NO_NAME = "file";

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        // A restored activity is one the system killed while the picker was up: its answer is
        // still on the way, and asking again would show a second picker behind it.
        if (savedInstanceState == null) {
            Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
            intent.addCategory(Intent.CATEGORY_OPENABLE);
            String type = getIntent().getStringExtra("type");
            intent.setType(type != null ? type : "*/*");
            try {
                try {
                startActivityForResult(intent, 1);
            } catch (ActivityNotFoundException error) {
                // A system with no Files app at all.
                writeError(new File(getCacheDir(), FOLDER), message(error));
                finish();
            }
            } catch (RuntimeException error) {
                // No document provider on this device answers: a transparent activity must not
                // stay up with nothing in it, and the game then sees no pick.
                finish();
            }
        }
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (resultCode != RESULT_OK || data == null || data.getData() == null) {
            // Nothing picked (or a picker that answered with no document): nothing to copy.
            finish();
            return;
        }
        Uri uri = data.getData();
        File folder = new File(getCacheDir(), FOLDER);
        new Thread(() -> {
            try {
                copy(uri, folder);
            } catch (IOException | RuntimeException error) {
                writeError(folder, message(error));
            } finally {
                runOnUiThread(this::finish);
            }
        }, "magnetite-pick").start();
    }

    /** Copies the picked document into {@code folder}, as {@code <name>}, or leaves error.txt. */
    private void copy(Uri uri, File folder) throws IOException {
        folder.mkdirs();
        // One pick at a time: the boot loop reads the folder, and an older copy is dead weight.
        File[] old = folder.listFiles();
        if (old != null) {
            for (File file : old) {
                file.delete();
            }
        }
        String name = plainName(displayName(uri));
        File part = new File(folder, name + ".part");
        File target = new File(folder, name);
        try {
            copyInto(uri, part);
            if (!part.renameTo(target)) {
                throw new IOException("cannot rename " + part);
            }
        } catch (IOException | RuntimeException error) {
            part.delete();
            throw error instanceof IOException
                    ? (IOException) error
                    : new IOException(message(error));
        }
    }

    /** The document's bytes into {@code part}, at most {@link #LIMIT_BYTES}. */
    private void copyInto(Uri uri, File part) throws IOException {
        InputStream in = getContentResolver().openInputStream(uri);
        if (in == null) {
            throw new IOException("cannot read " + uri);
        }
        try (InputStream source = in; OutputStream sink = new FileOutputStream(part)) {
            byte[] buffer = new byte[64 * 1024];
            long total = 0;
            int read;
            while ((read = source.read(buffer)) != -1) {
                total += read;
                if (total > LIMIT_BYTES) {
                    throw new IOException("the file is larger than 256 MiB");
                }
                sink.write(buffer, 0, read);
            }
        }
    }

    /** The picked document's display name, or null when the provider names none. */
    private String displayName(Uri uri) {
        try (Cursor cursor = getContentResolver().query(uri, null, null, null, null)) {
            if (cursor == null) {
                return null;
            }
            int column = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME);
            if (column < 0 || !cursor.moveToFirst()) {
                return null;
            }
            return cursor.getString(column);
        } catch (RuntimeException error) {
            return null;
        }
    }

    /**
     * A display name as a file name in the folder: [A-Za-z0-9._-] and nothing else, at most
     * {@link #NAME_LIMIT} characters, never starting with '.', and {@link #NO_NAME} when nothing
     * is left.
     */
    static String plainName(String displayName) {
        StringBuilder name = new StringBuilder();
        if (displayName != null) {
            for (int i = 0; i < displayName.length() && name.length() < NAME_LIMIT; i++) {
                char c = displayName.charAt(i);
                boolean plain = (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z')
                        || (c >= '0' && c <= '9') || c == '.' || c == '_' || c == '-';
                name.append(plain ? c : '_');
            }
        }
        // A leading dot hides the file, and the folder is the player's cache.
        if (name.length() > 0 && name.charAt(0) == '.') {
            name.setCharAt(0, '_');
        }
        if (name.length() == 0) {
            return NO_NAME;
        }
        if (name.toString().equals(ERROR)) {
            return "_" + ERROR;
        }
        // The Rust side skips names ending in ".part" as copies still running (pick.rs): a
        // document actually named that would never be seen.
        if (name.toString().endsWith(".part")) {
            if (name.length() >= NAME_LIMIT) {
                name.setLength(NAME_LIMIT - 1);
            }
            name.append('_');
        }
        return name.toString();
    }

    /** The reason for a failed pick into {@code folder}/error.txt, the same two-file way. */
    private static void writeError(File folder, String message) {
        folder.mkdirs();
        File part = new File(folder, ERROR + ".part");
        File target = new File(folder, ERROR);
        try (OutputStream sink = new FileOutputStream(part)) {
            sink.write(message.getBytes(StandardCharsets.UTF_8));
        } catch (IOException ignored) {
            // No folder to report into: the boot loop then sees no pick at all.
            return;
        }
        part.renameTo(target);
    }

    /** An error's text: its message, or its own words when it has none. */
    private static String message(Throwable error) {
        String message = error.getMessage();
        return message != null ? message : error.toString();
    }
}
