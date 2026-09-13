using System.IO;
using Microsoft.Data.Sqlite;

namespace ClinicDesk;

internal static class Store
{
    private static string Folder => Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        "ClinicDesk", "data");

    private static string FilePath => Path.Combine(Folder, "clinic.sqlite3");

    public static void Start()
    {
        Directory.CreateDirectory(Folder);
        using var db = Open();
        using var cmd = db.CreateCommand();
        cmd.CommandText =
            "CREATE TABLE IF NOT EXISTS people (" +
            "id INTEGER PRIMARY KEY AUTOINCREMENT," +
            "code TEXT NOT NULL UNIQUE," +
            "name TEXT NOT NULL," +
            "phone TEXT NOT NULL," +
            "nid TEXT NOT NULL DEFAULT '');" +
            "CREATE TABLE IF NOT EXISTS visits (" +
            "id INTEGER PRIMARY KEY AUTOINCREMENT," +
            "person_id INTEGER NOT NULL," +
            "when_text TEXT NOT NULL," +
            "doctor TEXT NOT NULL," +
            "state TEXT NOT NULL," +
            "note TEXT);" +
            "CREATE TABLE IF NOT EXISTS doctors (" +
            "id INTEGER PRIMARY KEY AUTOINCREMENT," +
            "name TEXT NOT NULL UNIQUE);" +
            "CREATE TABLE IF NOT EXISTS prefs (" +
            "k TEXT PRIMARY KEY," +
            "v TEXT NOT NULL);" +
            "CREATE INDEX IF NOT EXISTS ix_visits_when ON visits(when_text);" +
            "INSERT OR IGNORE INTO prefs(k,v) VALUES('open','08:00');" +
            "INSERT OR IGNORE INTO prefs(k,v) VALUES('close','17:00');" +
            "INSERT OR IGNORE INTO doctors(name) VALUES('طبيب عام');";
        cmd.ExecuteNonQuery();
        try
        {
            using var alter = db.CreateCommand();
            alter.CommandText = "ALTER TABLE people ADD COLUMN nid TEXT NOT NULL DEFAULT '';";
            alter.ExecuteNonQuery();
        }
        catch (SqliteException) { }
    }

    public static SqliteConnection Open()
    {
        var db = new SqliteConnection(new SqliteConnectionStringBuilder
        {
            DataSource = FilePath,
            Mode = SqliteOpenMode.ReadWriteCreate
        }.ToString());
        db.Open();
        return db;
    }
}
