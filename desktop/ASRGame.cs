// Windows setup, stable bootstrap, and versioned maintenance application. No external runtime packages.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Net;
using System.Reflection;
using System.Security.Cryptography;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows.Forms;
using Microsoft.Win32;

namespace ASRDesktop {
    public sealed class OwnedFile { public string path; public long size; public string sha256; }
    public sealed class Package { public int schema; public string product; public string version; public List<OwnedFile> files; }
    public sealed class Update { public int schema; public string product; public string version; public string archive; public long size; public string sha256; }
    public sealed class Current { public string version; public string directory; }
    public sealed class Installation { public int schema; public string product; public string repository; public List<OwnedFile> data; }
    public sealed class ReleaseAsset { public string name; public string browser_download_url; }
    public sealed class GitHubRelease { public string tag_name; public bool draft; public bool prerelease; public List<ReleaseAsset> assets; }

    static class App {
        internal const string Product = "asr-game-desktop", Title = "ASR Game";
        internal const string RegistryPath = @"Software\Microsoft\Windows\CurrentVersion\Uninstall\ASRGameDesktop";
        internal static readonly string Self = Assembly.GetExecutingAssembly().Location;
        internal static readonly string UserHome = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "ASRGame");
        internal static readonly JavaScriptSerializer Json = new JavaScriptSerializer { MaxJsonLength = 4 * 1024 * 1024 };
        internal static string Q(string text) { if (text.IndexOf('"') >= 0) throw new InvalidDataException("Invalid path."); return "\"" + text.TrimEnd('\\') + "\""; }
        internal static string Arg(string[] args, string flag) { int i = Array.IndexOf(args, flag); return i >= 0 && i + 1 < args.Length ? args[i + 1] : null; }
        internal static T Read<T>(string path) { return Json.Deserialize<T>(File.ReadAllText(path, Encoding.UTF8)); }
        internal static string Hash(string path) { using (var file = File.OpenRead(path)) using (var sha = SHA256.Create()) return BitConverter.ToString(sha.ComputeHash(file)).Replace("-", "").ToLowerInvariant(); }
        internal static void Log(string message) { try { Directory.CreateDirectory(UserHome); File.AppendAllText(Path.Combine(UserHome, "updates.log"), DateTime.UtcNow.ToString("o") + " " + message + Environment.NewLine); } catch { } }
        internal static void Atomic<T>(string path, T value) {
            Directory.CreateDirectory(Path.GetDirectoryName(path));
            string temporary = path + ".new-" + Guid.NewGuid().ToString("N");
            File.WriteAllText(temporary, Json.Serialize(value), new UTF8Encoding(false));
            if (File.Exists(path)) File.Replace(temporary, path, null); else File.Move(temporary, path);
        }
        internal static Version VersionOf(string text) {
            Version value;
            if (text == null || !Regex.IsMatch(text, @"^\d{1,5}\.\d{1,5}\.\d{1,5}\.\d{1,5}$") || !Version.TryParse(text, out value)) throw new InvalidDataException("Invalid release version.");
            return value;
        }
        internal static string Under(string root, string relative) {
            if (String.IsNullOrEmpty(relative) || relative.IndexOf(':') >= 0 || relative.IndexOf('\\') >= 0 || relative.StartsWith("/") || relative.Split('/').Any(p => p == "." || p == ".." || p.Length == 0)) throw new InvalidDataException("Invalid package path.");
            string prefix = Path.GetFullPath(root).TrimEnd('\\') + "\\";
            string result = Path.GetFullPath(Path.Combine(root, relative.Replace('/', '\\')));
            if (!result.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)) throw new InvalidDataException("Package path escapes its folder.");
            return result;
        }
        internal static Installation Managed(string root) {
            var install = Read<Installation>(Path.Combine(root, "install.json"));
            if (install == null || install.schema != 1 || install.product != Product || install.repository != ReleaseInfo.Repository) throw new InvalidDataException("This is not an ASR Game installation.");
            return install;
        }
        internal static Mutex Gate(string root) { return new Mutex(false, "Local\\ASRGame-Update-" + HashText(Path.GetFullPath(root).TrimEnd('\\').ToLowerInvariant())); }
        internal static bool Enter(Mutex gate) { try { return gate.WaitOne(0); } catch (AbandonedMutexException) { return true; } }
        internal static Current Pointer(string root) {
            Managed(root);
            var current = Read<Current>(Path.Combine(root, "current.json"));
            if (current == null || current.directory == null || !Regex.IsMatch(current.directory, @"^[0-9A-Za-z.-]+$") || current.directory == "." || current.directory == "..") throw new InvalidDataException("The installation needs repair. Run Setup again.");
            VersionOf(current.version); return current;
        }
        internal static string VersionFolder(string root, Current pointer) { return Under(Path.Combine(root, "versions"), pointer.directory); }
        internal static bool PlainPath(string root, string path) {
            string boundary = Path.GetFullPath(root).TrimEnd('\\');
            string full = Path.GetFullPath(path);
            if (!full.StartsWith(boundary + "\\", StringComparison.OrdinalIgnoreCase)) return false;
            for (string at = full; at != null; at = Path.GetDirectoryName(at)) {
                if ((File.Exists(at) || Directory.Exists(at)) && (File.GetAttributes(at) & FileAttributes.ReparsePoint) != 0) return false;
                if (at.Equals(boundary, StringComparison.OrdinalIgnoreCase)) return true;
            }
            return false;
        }
        internal static Dictionary<int, string> TalentLines(string text) {
            var result = new Dictionary<int, string>();
            foreach (string line in text.Split('\n')) {
                string[] words = line.Split((char[])null, StringSplitOptions.RemoveEmptyEntries); int id; double xp;
                if (words.Length < 2 || !Int32.TryParse(words[0], out id) || id < 1 || id > 255 || !Double.TryParse(words[1], System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out xp) || Double.IsNaN(xp) || Double.IsInfinity(xp) || xp < 0) continue;
                result[id] = line.Trim();
            }
            return result;
        }
        internal static double TalentXp(string line) { return Double.Parse(line.Split((char[])null, StringSplitOptions.RemoveEmptyEntries)[1], System.Globalization.CultureInfo.InvariantCulture); }
        internal static void RecoverSaves(string root) {
            string home = Path.Combine(UserHome, "benilla-config"); Directory.CreateDirectory(home);
            var sources = new List<string> { Path.Combine(root, "benilla-config") };
            string versions = Path.Combine(root, "versions");
            if (Directory.Exists(versions)) foreach (string folder in Directory.GetDirectories(versions)) if (PlainPath(root, folder)) sources.Add(Path.Combine(folder, "benilla-config"));
            foreach (string source in sources) {
                if (!Directory.Exists(source) || !PlainPath(root, source)) continue;
                foreach (string file in Directory.GetFiles(source)) {
                    if (!PlainPath(source, file)) continue;
                    string name = Path.GetFileName(file), target = Path.Combine(home, name);
                    if (!File.Exists(target)) { File.Copy(file, target, false); Log("Recovered save " + name + " from " + source); }
                    else if (name.Equals("survivors-talents.txt", StringComparison.OrdinalIgnoreCase)) {
                        var current = TalentLines(File.ReadAllText(target)); var old = TalentLines(File.ReadAllText(file)); bool changed = false;
                        foreach (var entry in old) if (!current.ContainsKey(entry.Key) || TalentXp(entry.Value) > TalentXp(current[entry.Key])) { current[entry.Key] = entry.Value; changed = true; }
                        if (changed) {
                            File.Copy(target, target + ".before-recovery-" + Guid.NewGuid().ToString("N"), false);
                            string temporary = target + ".new-" + Guid.NewGuid().ToString("N");
                            File.WriteAllText(temporary, String.Join("\n", current.OrderBy(p => p.Key).Select(p => p.Value)), new UTF8Encoding(false));
                            File.Replace(temporary, target, null); Log("Recovered higher class talent progress from " + source);
                        }
                    }
                }
            }
        }
        internal static void PruneVersions(string root) {
            Managed(root); var current = Pointer(root); string versions = Path.Combine(root, "versions");
            if (!PlainPath(root, versions) || !Directory.Exists(versions) || RunningGame(root)) return;
            RecoverSaves(root);
            var keep = new HashSet<string>(StringComparer.OrdinalIgnoreCase) { VersionFolder(root, current), Path.GetDirectoryName(Self) };
            string previous = Path.Combine(root, "previous.json");
            if (File.Exists(previous)) keep.Add(VersionFolder(root, Read<Current>(previous)));
            foreach (string folder in Directory.GetDirectories(versions)) {
                if (keep.Contains(folder) || !PlainPath(root, folder) || !Regex.IsMatch(Path.GetFileName(folder), @"^\d+\.\d+\.\d+\.\d+-[0-9a-f]{8}$")) continue;
                try {
                    string manifest = Path.Combine(folder, "package.json"); if (!PlainPath(folder, manifest) || !File.Exists(manifest)) continue;
                    var package = Read<Package>(manifest);
                    if (package == null || package.schema != 1 || package.product != Product || package.files == null || package.version + "-" != Path.GetFileName(folder).Substring(0, Path.GetFileName(folder).Length - 8)) continue;
                    var owned = new HashSet<string>(StringComparer.OrdinalIgnoreCase) { manifest };
                    bool safe = true;
                    foreach (var file in package.files) {
                        string path = Under(folder, file.path);
                        if (!PlainPath(folder, path) || file.path.StartsWith("benilla-config", StringComparison.OrdinalIgnoreCase) || file.path.StartsWith("Data/", StringComparison.OrdinalIgnoreCase)) { safe = false; break; }
                        owned.Add(path);
                        if (File.Exists(path)) ValidateFile(folder, file);
                    }
                    if (!safe) continue;
                    // Never recursively delete a version folder: remove only unchanged,
                    // manifest-owned files. Unknown files and local saves remain intact.
                    foreach (string path in owned.Where(p => !p.Equals(manifest, StringComparison.OrdinalIgnoreCase))) if (File.Exists(path)) File.Delete(path);
                    File.Delete(manifest);
                    foreach (string path in owned.Select(Path.GetDirectoryName).Distinct().OrderByDescending(p => p.Length)) {
                        if (PlainPath(root, path) && Directory.Exists(path) && Directory.GetFileSystemEntries(path).Length == 0) Directory.Delete(path);
                    }
                    Log("Pruned obsolete version " + Path.GetFileName(folder));
                } catch (Exception error) { Log("Version cleanup skipped " + folder + ": " + error.Message); }
            }
        }
        internal static string Worker(string root) {
            var current = Pointer(root); string folder = VersionFolder(root, current);
            var package = Read<Package>(Path.Combine(folder, "package.json"));
            if (package == null || package.product != Product || package.files == null) throw new InvalidDataException("The installed launcher needs repair.");
            ValidateFile(folder, package.files.Single(f => f.path == "ASRGame.exe"));
            return Path.Combine(folder, "ASRGame.exe");
        }
        internal static bool RunningGame(string root) {
            string prefix = Path.GetFullPath(root).TrimEnd('\\') + "\\";
            foreach (string name in new[] { "warcraft-survivors", "asr-coop" }) foreach (var process in Process.GetProcessesByName(name)) {
                using (process) { try { if (process.MainModule.FileName.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)) return true; } catch { return true; } }
            }
            return false;
        }
        internal static void ValidateFile(string root, OwnedFile file) {
            if (file == null || file.size < 0 || file.sha256 == null || !Regex.IsMatch(file.sha256, "^[a-fA-F0-9]{64}$")) throw new InvalidDataException("Invalid file manifest.");
            string path = Under(root, file.path);
            if (!File.Exists(path) || new FileInfo(path).Length != file.size || !Hash(path).Equals(file.sha256, StringComparison.OrdinalIgnoreCase)) throw new InvalidDataException("File verification failed: " + file.path);
        }
        internal static Package Extract(string archive, string destination, bool permitData, Action<string, int> progress) {
            Directory.CreateDirectory(destination);
            using (var zip = ZipFile.OpenRead(archive)) {
                if (zip.Entries.Count > 2048) throw new InvalidDataException("Too many package entries.");
                var metadata = zip.GetEntry("package.json");
                if (metadata == null || metadata.Length > 2 * 1024 * 1024) throw new InvalidDataException("Package manifest missing.");
                Package package;
                using (var reader = new StreamReader(metadata.Open(), Encoding.UTF8)) package = Json.Deserialize<Package>(reader.ReadToEnd());
                if (package == null || package.schema != 1 || package.product != Product || package.files == null) throw new InvalidDataException("Unsupported package.");
                VersionOf(package.version);
                var allowed = new Dictionary<string, OwnedFile>(StringComparer.OrdinalIgnoreCase);
                long total = 0;
                foreach (var file in package.files) {
                    Under(destination, file.path);
                    if (file.path == "package.json" || allowed.ContainsKey(file.path) || file.size < 0 || file.size > 2L * 1024 * 1024 * 1024) throw new InvalidDataException("Invalid package entry.");
                    if (!permitData && file.path.StartsWith("Data/", StringComparison.OrdinalIgnoreCase)) throw new InvalidDataException("Runtime updates cannot replace game data.");
                    if (file.path.StartsWith("benilla-config", StringComparison.OrdinalIgnoreCase) || file.path == "install.json" || file.path == "current.json") throw new InvalidDataException("Updates cannot replace personal state.");
                    allowed.Add(file.path, file); total = checked(total + file.size);
                }
                if (total > 4L * 1024 * 1024 * 1024 || !allowed.ContainsKey("ASRGame.exe") || !allowed.ContainsKey("warcraft-survivors.exe") || zip.Entries.Count != allowed.Count + 1) throw new InvalidDataException("Incomplete or oversized package.");
                long written = 0;
                foreach (var entry in zip.Entries) {
                    if (entry.FullName == "package.json") continue;
                    OwnedFile file;
                    if (!allowed.TryGetValue(entry.FullName, out file) || entry.Length != file.size) throw new InvalidDataException("Unexpected package entry.");
                    string path = Under(destination, file.path);
                    Directory.CreateDirectory(Path.GetDirectoryName(path));
                    using (var input = entry.Open()) using (var output = new FileStream(path, FileMode.CreateNew, FileAccess.Write)) {
                        byte[] buffer = new byte[1024 * 1024]; int count; long size = 0;
                        while ((count = input.Read(buffer, 0, buffer.Length)) > 0) { size += count; if (size > file.size) throw new InvalidDataException("Expanded entry exceeds its manifest."); output.Write(buffer, 0, count); written += count; progress("Installing " + Path.GetFileName(path), (int)(written * 100 / Math.Max(1, total))); }
                    }
                    ValidateFile(destination, file);
                }
                Atomic(Path.Combine(destination, "package.json"), package); return package;
            }
        }
        internal static string Activate(string root, string staged, Package package) {
            string directory = package.version + "-" + Guid.NewGuid().ToString("N").Substring(0, 8);
            Directory.CreateDirectory(Path.Combine(root, "versions"));
            string final = Under(Path.Combine(root, "versions"), directory);
            Directory.Move(staged, final);
            string pointer = Path.Combine(root, "current.json");
            if (File.Exists(pointer)) Atomic(Path.Combine(root, "previous.json"), Read<Current>(pointer));
            Atomic(pointer, new Current { version = package.version, directory = directory });
            return final;
        }
        internal static void Launch(string root) {
            var pointer = Pointer(root); string folder = VersionFolder(root, pointer);
            string data = Path.Combine(root, "Data");
            if (!Directory.Exists(data)) throw new DirectoryNotFoundException("Game data is missing. Run ASR Game Setup to repair the installation.");
            string home = Path.Combine(UserHome, "benilla-config");
            // Recover portable/version-local saves even if the shared folder already exists.
            // Do not touch a live game's save file; it will save its in-memory book at exit.
            if (!RunningGame(root)) { try { RecoverSaves(root); } catch (Exception error) { Log("Save recovery: " + error.Message); } }
            Directory.CreateDirectory(home);
            var info = new ProcessStartInfo(Path.Combine(folder, "warcraft-survivors.exe")) { WorkingDirectory = root, UseShellExecute = false };
            info.EnvironmentVariables["WOW_DATA"] = data;
            info.EnvironmentVariables["BENILLA_HOME"] = home;
            info.EnvironmentVariables["ASR_GAME_INSTALL_ROOT"] = root;
            Process.Start(info);
        }
        internal static void CopyTree(string source, string target) {
            Directory.CreateDirectory(target);
            foreach (var file in Directory.GetFiles(source)) {
                if ((File.GetAttributes(file) & FileAttributes.ReparsePoint) != 0) continue;
                File.Copy(file, Path.Combine(target, Path.GetFileName(file)), false);
            }
            foreach (string child in Directory.GetDirectories(source)) if ((File.GetAttributes(child) & FileAttributes.ReparsePoint) == 0) CopyTree(child, Path.Combine(target, Path.GetFileName(child)));
        }
        internal static void Shortcut(string path, string executable, string arguments, string working) {
            Directory.CreateDirectory(Path.GetDirectoryName(path));
            Type type = Type.GetTypeFromProgID("WScript.Shell"); object shell = Activator.CreateInstance(type);
            object shortcut = type.InvokeMember("CreateShortcut", BindingFlags.InvokeMethod, null, shell, new object[] { path });
            Type st = shortcut.GetType();
            foreach (var pair in new[] { new[] {"TargetPath", executable}, new[] {"Arguments", arguments}, new[] {"WorkingDirectory", working}, new[] {"Description", Title} }) st.InvokeMember(pair[0], BindingFlags.SetProperty, null, shortcut, new object[] { pair[1] });
            st.InvokeMember("Save", BindingFlags.InvokeMethod, null, shortcut, null);
            System.Runtime.InteropServices.Marshal.FinalReleaseComObject(shortcut); System.Runtime.InteropServices.Marshal.FinalReleaseComObject(shell);
        }
        internal static void Register(string root, string version, bool desktop) {
            string executable = Path.Combine(root, "ASRGame.exe");
            string menu = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Programs), Title);
            Shortcut(Path.Combine(menu, Title + ".lnk"), executable, "--play", root);
            Shortcut(Path.Combine(menu, "Check for updates.lnk"), executable, "--check-update", root);
            Shortcut(Path.Combine(menu, "Uninstall.lnk"), executable, "--uninstall", root);
            if (desktop) Shortcut(Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory), Title + ".lnk"), executable, "--play", root);
            using (var key = Registry.CurrentUser.CreateSubKey(RegistryPath)) {
                key.SetValue("DisplayName", Title); key.SetValue("DisplayVersion", version); key.SetValue("Publisher", "calinadrian");
                key.SetValue("InstallLocation", root); key.SetValue("DisplayIcon", executable);
                key.SetValue("UninstallString", Q(executable) + " --uninstall"); key.SetValue("NoModify", 1); key.SetValue("NoRepair", 1);
            }
        }
        internal static void WaitForParent(string[] args) {
            int id; if (!Int32.TryParse(Arg(args, "--parent"), out id)) return;
            try { using (var process = Process.GetProcessById(id)) { if (!process.WaitForExit(30000)) throw new IOException("The launcher is still running."); } } catch (ArgumentException) { }
        }
        internal static void Uninstall(string root) {
            using (var gate = Gate(root)) {
                if (!Enter(gate)) throw new IOException("An ASR Game update is running. Try uninstalling again when it finishes.");
                try { UninstallLocked(root); } finally { gate.ReleaseMutex(); }
            }
        }
        static void UninstallLocked(string root) {
            var installation = Managed(root);
            if (RunningGame(root)) throw new IOException("Close ASR Game and its co-op server before uninstalling it.");
            if (MessageBox.Show("Remove ASR Game? Your characters, talents and settings will be kept.", Title, MessageBoxButtons.YesNo, MessageBoxIcon.Question) != DialogResult.Yes) return;
            string versions = Path.Combine(root, "versions");
            if (Directory.Exists(versions)) foreach (string folder in Directory.GetDirectories(versions)) {
                if ((File.GetAttributes(folder) & FileAttributes.ReparsePoint) != 0 || !File.Exists(Path.Combine(folder, "package.json"))) continue;
                var package = Read<Package>(Path.Combine(folder, "package.json"));
                if (package == null || package.product != Product || package.files == null) continue;
                foreach (var file in package.files) { string path = Under(folder, file.path); if (File.Exists(path)) File.Delete(path); }
                File.Delete(Path.Combine(folder, "package.json")); RemoveEmpty(folder);
            }
            if (installation.data != null) foreach (var file in installation.data) {
                string path = Under(root, file.path);
                if (!file.path.StartsWith("Data/", StringComparison.OrdinalIgnoreCase)) throw new InvalidDataException("Invalid owned data path.");
                if (File.Exists(path) && Hash(path).Equals(file.sha256, StringComparison.OrdinalIgnoreCase)) File.Delete(path);
            }
            foreach (var name in new[] {"ASRGame.exe", "current.json", "previous.json", "install.json"}) { string path = Path.Combine(root, name); if (File.Exists(path)) File.Delete(path); }
            string menu = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Programs), Title);
            foreach (var name in new[] { Title + ".lnk", "Check for updates.lnk", "Uninstall.lnk" }) { string path = Path.Combine(menu, name); if (File.Exists(path)) File.Delete(path); }
            string desktop = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory), Title + ".lnk"); if (File.Exists(desktop)) File.Delete(desktop);
            Registry.CurrentUser.DeleteSubKeyTree(RegistryPath, false); RemoveEmpty(menu); RemoveEmpty(root);
            MessageBox.Show("ASR Game was removed. Your saves are kept in " + UserHome, Title);
        }
        internal static void RemoveEmpty(string root) {
            if (!Directory.Exists(root) || (File.GetAttributes(root) & FileAttributes.ReparsePoint) != 0) return;
            foreach (var directory in Directory.GetDirectories(root)) RemoveEmpty(directory);
            if (Directory.GetFileSystemEntries(root).Length == 0) Directory.Delete(root);
        }

        [STAThread] static void Main(string[] args) {
            Application.EnableVisualStyles(); Application.SetCompatibleTextRenderingDefault(false);
            ServicePointManager.SecurityProtocol = SecurityProtocolType.Tls12;
            try {
                string root = Arg(args, "--root") ?? Path.GetDirectoryName(Self);
                if (args.Contains("--uninstall-worker")) { WaitForParent(args); Uninstall(root); return; }
                if (args.Contains("--uninstall")) {
                    if (!args.Contains("--worker")) {
                        Process.Start(new ProcessStartInfo(Worker(root), "--uninstall --worker --root " + Q(root)) { UseShellExecute = false }); return;
                    }
                    Managed(root); string temporary = Path.Combine(Path.GetTempPath(), "ASRGame-Uninstall-" + Guid.NewGuid().ToString("N") + ".exe"); File.Copy(Self, temporary);
                    Process.Start(new ProcessStartInfo(temporary, "--uninstall-worker --root " + Q(root) + " --parent " + Process.GetCurrentProcess().Id) { UseShellExecute = false }); return;
                }
                bool play = args.Contains("--play"), check = args.Contains("--check-update");
                if (!play && !check && (Path.GetFileName(Self).EndsWith("-Setup.exe", StringComparison.OrdinalIgnoreCase) || !File.Exists(Path.Combine(root, "install.json")))) { Application.Run(new SetupWindow()); return; }
                if (!args.Contains("--worker")) {
                    string worker = Worker(root);
                    Process.Start(new ProcessStartInfo(worker, (check ? "--check-update" : "--play") + " --worker --root " + Q(root)) { UseShellExecute = false, WorkingDirectory = root }); return;
                }
                Managed(root);
                using (var mutex = Gate(root)) {
                    if (!Enter(mutex)) { if (play) Launch(root); else MessageBox.Show("ASR Game is already checking for updates.", Title); return; }
                    try {
                        try { PruneVersions(root); } catch (Exception error) { Log("Version cleanup: " + error.Message); }
                        Application.Run(new UpdateWindow(root, play || !check));
                    } finally { mutex.ReleaseMutex(); }
                }
            } catch (Exception error) { Log(error.ToString()); MessageBox.Show(error.Message, Title, MessageBoxButtons.OK, MessageBoxIcon.Error); }
        }
        internal static string HashText(string text) { using (var sha = SHA256.Create()) return BitConverter.ToString(sha.ComputeHash(Encoding.UTF8.GetBytes(text))).Replace("-", "").Substring(0, 24); }
    }

    static class Download {
        internal static HttpWebResponse Open(string url) {
            Uri uri; if (!Uri.TryCreate(url, UriKind.Absolute, out uri) || uri.Scheme != "https" || !Allowed(uri.Host)) throw new InvalidDataException("Unexpected update URL.");
            var request = (HttpWebRequest)WebRequest.Create(uri); request.UserAgent = "ASRGame/" + ReleaseInfo.Version; request.Accept = "application/vnd.github+json";
            request.Timeout = 8000; request.ReadWriteTimeout = 60000;
            var response = (HttpWebResponse)request.GetResponse();
            if (response.ResponseUri.Scheme != "https" || !Allowed(response.ResponseUri.Host)) { response.Dispose(); throw new InvalidDataException("Unexpected update redirect."); }
            return response;
        }
        static bool Allowed(string host) { return host == "github.com" || host == "api.github.com" || host == "release-assets.githubusercontent.com" || host == "objects.githubusercontent.com"; }
        internal static T Json<T>(string url) {
            using (var response = Open(url)) using (var input = response.GetResponseStream()) using (var output = new MemoryStream()) {
                byte[] buffer = new byte[16384]; int count;
                while ((count = input.Read(buffer, 0, buffer.Length)) > 0) { if (output.Length + count > 4 * 1024 * 1024) throw new InvalidDataException("Update metadata exceeds its limit."); output.Write(buffer, 0, count); }
                return App.Json.Deserialize<T>(Encoding.UTF8.GetString(output.ToArray()));
            }
        }
        internal static Update Latest(out string baseUrl) {
            baseUrl = null;
            var releases = Json<List<GitHubRelease>>("https://api.github.com/repos/" + ReleaseInfo.Repository + "/releases?per_page=100");
            var candidates = releases.Where(r => !r.draft && !r.prerelease && r.tag_name != null && Regex.IsMatch(r.tag_name, @"^desktop-v\d+\.\d+\.\d+\.\d+$"))
                .OrderByDescending(r => App.VersionOf(r.tag_name.Substring(9)));
            foreach (var release in candidates) {
                string prefix = "https://github.com/" + ReleaseInfo.Repository + "/releases/download/" + release.tag_name + "/";
                if (release.assets == null || !release.assets.Any(a => a.name == "update.json" && a.browser_download_url == prefix + "update.json")) continue;
                var update = Json<Update>(prefix + "update.json");
                if (update == null || update.schema != 1 || update.product != App.Product || update.version != release.tag_name.Substring(9)
                    || update.archive != "ASRGame-" + update.version + "-win-x64.zip" || update.size <= 0 || update.size > 1024L * 1024 * 1024
                    || update.sha256 == null || !Regex.IsMatch(update.sha256, "^[a-fA-F0-9]{64}$")
                    || !release.assets.Any(a => a.name == update.archive && a.browser_download_url == prefix + update.archive)) throw new InvalidDataException("Invalid desktop release manifest.");
                baseUrl = prefix; return update;
            }
            return null;
        }
        internal static void Archive(string url, string path, Update update, Action<string, int> progress) {
            using (var response = Open(url)) using (var input = response.GetResponseStream()) using (var output = new FileStream(path, FileMode.CreateNew, FileAccess.Write)) {
                if (response.ContentLength >= 0 && response.ContentLength != update.size) throw new InvalidDataException("Unexpected download size.");
                byte[] buffer = new byte[1024 * 1024]; int count; long done = 0;
                while ((count = input.Read(buffer, 0, buffer.Length)) > 0) { done += count; if (done > update.size) throw new InvalidDataException("Update exceeds its manifest size."); output.Write(buffer, 0, count); progress("Downloading update…", (int)(done * 100 / update.size)); }
                if (done != update.size) throw new InvalidDataException("Download was interrupted.");
            }
            if (!App.Hash(path).Equals(update.sha256, StringComparison.OrdinalIgnoreCase)) throw new InvalidDataException("The update failed its SHA-256 check.");
        }
    }

    class Window : Form {
        protected readonly Label Status = new Label(); protected readonly ProgressBar Progress = new ProgressBar();
        protected Window(string heading, string subtitle) {
            Text = App.Title; ClientSize = new Size(570, 370); FormBorderStyle = FormBorderStyle.FixedDialog; MaximizeBox = false; MinimizeBox = false; StartPosition = FormStartPosition.CenterScreen;
            BackColor = Color.FromArgb(25, 24, 21); ForeColor = Color.FromArgb(235, 224, 197); Font = new Font("Segoe UI", 10);
            Icon = Icon.ExtractAssociatedIcon(App.Self);
            Controls.Add(new Label { Text = heading, Font = new Font("Segoe UI", 22, FontStyle.Bold), ForeColor = Color.FromArgb(235, 191, 102), AutoSize = true, Location = new Point(26, 22) });
            Controls.Add(new Label { Text = subtitle, Location = new Point(28, 76), Size = new Size(510, 40) });
            Status.SetBounds(28, 270, 510, 42); Controls.Add(Status); Progress.SetBounds(28, 318, 510, 8); Controls.Add(Progress);
        }
        protected Button Button(string title, int left, int top, int width, EventHandler click) {
            var button = new Button { Text = title, Location = new Point(left, top), Size = new Size(width, 37), FlatStyle = FlatStyle.Flat, BackColor = Color.FromArgb(77, 30, 20), ForeColor = Color.FromArgb(244, 206, 128) };
            button.FlatAppearance.BorderColor = Color.FromArgb(138, 108, 51); button.Click += click; Controls.Add(button); return button;
        }
        protected void Report(string text, int percent) { if (!IsDisposed && IsHandleCreated) BeginInvoke((Action)(() => { Status.Text = text; Progress.Value = Math.Max(0, Math.Min(100, percent)); })); }
    }

    sealed class UpdateWindow : Window {
        readonly string root; readonly bool play; bool busy;
        internal UpdateWindow(string root, bool play) : base("ASR GAME", "Checking GitHub for updates…") {
            this.root = root; this.play = play; ShowInTaskbar = !play; Opacity = play ? 0 : 1;
            FormClosing += (s, e) => { if (busy) e.Cancel = true; };
            Shown += async (s, e) => await Run();
        }
        async Task Run() {
            string staging = null, archive = null;
            try {
                var current = App.Pointer(root); string baseUrl = null;
                var update = await Task.Run(() => Download.Latest(out baseUrl));
                if (update != null && App.VersionOf(update.version) > App.VersionOf(current.version)) {
                    Opacity = 1; ShowInTaskbar = true;
                    if (MessageBox.Show(this, "ASR Game " + update.version + " is available.\n\nDownload and install the update now? Your progress is kept.", App.Title, MessageBoxButtons.YesNo, MessageBoxIcon.Information) == DialogResult.Yes) {
                        busy = true; Status.Text = "Preparing update…";
                        staging = Path.Combine(root, "versions", ".staging-" + Guid.NewGuid().ToString("N")); Directory.CreateDirectory(Path.GetDirectoryName(staging));
                        archive = Path.Combine(root, ".download-" + Guid.NewGuid().ToString("N") + ".zip");
                        await Task.Run(() => {
                            Download.Archive(baseUrl + update.archive, archive, update, Report);
                            var package = App.Extract(archive, staging, false, Report);
                            if (package.version != update.version) throw new InvalidDataException("Update and package versions differ.");
                            App.Activate(root, staging, package);
                            using (var key = Registry.CurrentUser.OpenSubKey(App.RegistryPath, true)) { if (key != null) key.SetValue("DisplayVersion", update.version); }
                        });
                        staging = null;
                        if (!play) MessageBox.Show(this, "ASR Game is up to date. The new version will open next time you launch the game.", App.Title);
                    }
                } else if (!play) MessageBox.Show(this, update == null ? "No desktop releases have been published yet." : "You're running the latest version.", App.Title);
            } catch (Exception error) {
                App.Log(error.ToString());
                if (!play || busy) { Opacity = 1; MessageBox.Show(this, "The update could not be completed. Your current installation is still available.\n\n" + error.Message, App.Title, MessageBoxButtons.OK, MessageBoxIcon.Warning); }
            } finally {
                busy = false;
                // Only the exact staging folder created by this update is eligible for cleanup.
                if (archive != null && File.Exists(archive)) { try { File.Delete(archive); } catch { } }
                if (staging != null) { try { SetupWindow.RemoveStaging(staging, Path.Combine(root, "versions")); } catch { } }
            }
            try { if (play) App.Launch(root); } catch (Exception error) { MessageBox.Show(this, error.Message, App.Title); }
            Close();
        }
    }

    sealed class SetupWindow : Window {
        readonly TextBox folder = new TextBox(); readonly CheckBox desktop = new CheckBox(); readonly Button install; bool busy;
        internal SetupWindow() : base("INSTALL ASR GAME", "Azeroth Survivors with co-op and GitHub updates.\nNo administrator rights are required.") {
            folder.SetBounds(28, 134, 405, 27); folder.Text = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Programs", "ASRGame");
            using (var key = Registry.CurrentUser.OpenSubKey(App.RegistryPath)) { if (key != null) folder.Text = (string)key.GetValue("InstallLocation", folder.Text); }
            Controls.Add(folder);
            Button("Browse…", 444, 130, 94, (s, e) => { using (var dialog = new FolderBrowserDialog { Description = "Choose an empty installation folder", SelectedPath = folder.Text }) if (dialog.ShowDialog(this) == DialogResult.OK) folder.Text = dialog.SelectedPath; });
            desktop.Text = "Create a desktop shortcut"; desktop.Checked = true; desktop.SetBounds(28, 177, 290, 28); Controls.Add(desktop);
            install = Button("Install", 355, 219, 183, async (s, e) => await Install());
            Button("Cancel", 239, 219, 104, (s, e) => Close());
            FormClosing += (s, e) => { if (busy) e.Cancel = true; };
        }
        static string Payload(string temporary) {
            string zip = Path.Combine(temporary, "setup.zip");
            using (var stream = File.OpenRead(App.Self)) {
                if (stream.Length < 17) throw new InvalidDataException("Installer payload is missing.");
                stream.Position = stream.Length - 17;
                using (var reader = new BinaryReader(stream, Encoding.ASCII, true)) {
                    long size = reader.ReadInt64(); string magic = Encoding.ASCII.GetString(reader.ReadBytes(9));
                    if (magic != "ASRSETUP1" || size <= 0 || size > stream.Length - 17) throw new InvalidDataException("Installer payload is missing or incomplete.");
                    stream.Position = stream.Length - 17 - size;
                    using (var output = File.Create(zip)) { byte[] buffer = new byte[1024 * 1024]; while (size > 0) { int count = stream.Read(buffer, 0, (int)Math.Min(buffer.Length, size)); if (count == 0) throw new EndOfStreamException(); output.Write(buffer, 0, count); size -= count; } }
                }
            }
            return zip;
        }
        async Task Install() {
            string root = null, temporary = null, staging = null;
            Installation previousInstall = null; bool activated = false, validatedRoot = false, metadataWritten = false, bootstrapCreated = false, gateHeld = false;
            Mutex gate = null;
            var dataWrites = new List<DataWrite>();
            try {
                root = Path.GetFullPath(folder.Text).TrimEnd('\\');
                if (root == Path.GetPathRoot(root).TrimEnd('\\') || root.Equals(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), StringComparison.OrdinalIgnoreCase)) throw new IOException("Choose a dedicated ASR Game folder.");
                foreach (var special in new[] { Environment.SpecialFolder.DesktopDirectory, Environment.SpecialFolder.MyDocuments, Environment.SpecialFolder.LocalApplicationData, Environment.SpecialFolder.Programs, Environment.SpecialFolder.ProgramFiles, Environment.SpecialFolder.ProgramFilesX86, Environment.SpecialFolder.Windows, Environment.SpecialFolder.System }) {
                    if (root.Equals(Environment.GetFolderPath(special).TrimEnd('\\'), StringComparison.OrdinalIgnoreCase)) throw new IOException("Choose an ASR Game subfolder, not a Windows or personal folder itself.");
                }
                if (Directory.Exists(root) && (File.GetAttributes(root) & FileAttributes.ReparsePoint) != 0) throw new IOException("Choose a folder that is not a directory link.");
                if (Directory.Exists(root) && Directory.GetFileSystemEntries(root).Length > 0) previousInstall = App.Managed(root);
                validatedRoot = true;
                gate = App.Gate(root); gateHeld = App.Enter(gate);
                if (!gateHeld) throw new IOException("An ASR Game update is running. Try setup again when it finishes.");
                if (App.RunningGame(root)) throw new IOException("Close ASR Game before installing or repairing it.");
                busy = true; install.Enabled = false; folder.Enabled = false;
                temporary = Path.Combine(Path.GetTempPath(), "ASRGame-Setup-" + Guid.NewGuid().ToString("N")); Directory.CreateDirectory(temporary);
                staging = Path.Combine(root, "versions", ".staging-" + Guid.NewGuid().ToString("N"));
                string payload = await Task.Run(() => Payload(temporary));
                var package = await Task.Run(() => App.Extract(payload, staging, true, Report));
                bool haveData = package.files.Any(f => f.path.StartsWith("Data/", StringComparison.OrdinalIgnoreCase));
                string source = null;
                if (!haveData && (!Directory.Exists(Path.Combine(root, "Data")) || Directory.GetFiles(Path.Combine(root, "Data"), "*.MPQ", SearchOption.AllDirectories).Length == 0)) {
                    using (var picker = new FolderBrowserDialog { Description = "Select your existing ASR Game Data folder (the folder containing your MPQ archives)." }) {
                        if (picker.ShowDialog(this) != DialogResult.OK) throw new OperationCanceledException("Installation cancelled. A Data folder is required."); source = picker.SelectedPath;
                    }
                }
                var owned = previousInstall != null && previousInstall.data != null ? new List<OwnedFile>(previousInstall.data) : new List<OwnedFile>();
                await Task.Run(() => {
                    foreach (var file in package.files.Where(f => f.path.StartsWith("Data/", StringComparison.OrdinalIgnoreCase)).ToList()) {
                        dataWrites.Add(DataWrite.Put(root, App.Under(staging, file.path), file));
                        File.Delete(App.Under(staging, file.path)); owned.RemoveAll(f => f.path.Equals(file.path, StringComparison.OrdinalIgnoreCase)); owned.Add(file);
                    }
                    if (source != null) {
                        var archives = Directory.GetFiles(source, "*.MPQ", SearchOption.AllDirectories);
                        if (archives.Length == 0) throw new IOException("The selected folder contains no MPQ archives.");
                        foreach (string file in archives) {
                            string relative = "Data/" + file.Substring(source.TrimEnd('\\').Length + 1).Replace('\\', '/');
                            var record = new OwnedFile { path = relative, size = new FileInfo(file).Length, sha256 = App.Hash(file) };
                            dataWrites.Add(DataWrite.Put(root, file, record)); owned.RemoveAll(f => f.path.Equals(relative, StringComparison.OrdinalIgnoreCase)); owned.Add(record); Report("Copying game data…", 95);
                        }
                    }
                    package.files.RemoveAll(f => f.path.StartsWith("Data/", StringComparison.OrdinalIgnoreCase));
                    App.Atomic(Path.Combine(staging, "package.json"), package);
                    Directory.CreateDirectory(root);
                    string bootstrap = Path.Combine(root, "ASRGame.exe");
                    if (!File.Exists(bootstrap)) { File.Copy(Path.Combine(staging, "ASRGame.exe"), bootstrap); bootstrapCreated = true; }
                    App.Atomic(Path.Combine(root, "install.json"), new Installation { schema = 1, product = App.Product, repository = ReleaseInfo.Repository, data = owned });
                    metadataWritten = true;
                    App.Activate(root, staging, package);
                });
                activated = true;
                foreach (var write in dataWrites) write.Commit();
                staging = null; App.Register(root, package.version, desktop.Checked);
                Progress.Value = 100; Status.Text = "Installed. Updates are checked automatically when you open ASR Game.";
                busy = false;
                if (MessageBox.Show(this, "ASR Game is installed. Play now?", App.Title, MessageBoxButtons.YesNo, MessageBoxIcon.Information) == DialogResult.Yes) Process.Start(new ProcessStartInfo(Path.Combine(root, "ASRGame.exe"), "--play") { UseShellExecute = false });
                Close();
            } catch (Exception error) { App.Log(error.ToString()); MessageBox.Show(this, error.Message, App.Title, MessageBoxButtons.OK, MessageBoxIcon.Warning); }
            finally {
                if (!activated) {
                    foreach (var write in Enumerable.Reverse(dataWrites)) { try { write.Rollback(); } catch (Exception error) { App.Log("Data rollback: " + error); } }
                    if (metadataWritten && root != null && File.Exists(Path.Combine(root, "install.json"))) {
                        try { if (previousInstall != null) App.Atomic(Path.Combine(root, "install.json"), previousInstall); else File.Delete(Path.Combine(root, "install.json")); } catch { }
                    }
                    if (previousInstall == null && bootstrapCreated) { try { File.Delete(Path.Combine(root, "ASRGame.exe")); } catch { } }
                }
                busy = false; install.Enabled = true; folder.Enabled = true;
                if (staging != null && root != null) { try { RemoveStaging(staging, Path.Combine(root, "versions")); } catch { } }
                if (temporary != null) { try { RemoveStaging(temporary, Path.GetTempPath()); } catch { } }
                if (!activated && validatedRoot && previousInstall == null) { try { App.RemoveEmpty(root); } catch { } }
                if (gate != null) { if (gateHeld) gate.ReleaseMutex(); gate.Dispose(); }
            }
        }
        sealed class DataWrite {
            string target, backup; bool changed;
            internal static DataWrite Put(string root, string source, OwnedFile file) {
                string target = App.Under(root, file.path);
                if (!file.path.StartsWith("Data/", StringComparison.OrdinalIgnoreCase)) throw new InvalidDataException("Invalid data destination.");
                var result = new DataWrite { target = target };
                if (File.Exists(target) && new FileInfo(target).Length == file.size && App.Hash(target).Equals(file.sha256, StringComparison.OrdinalIgnoreCase)) return result;
                Directory.CreateDirectory(Path.GetDirectoryName(target));
                string temporary = target + ".install-" + Guid.NewGuid().ToString("N");
                try {
                    File.Copy(source, temporary, false);
                    if (new FileInfo(temporary).Length != file.size || !App.Hash(temporary).Equals(file.sha256, StringComparison.OrdinalIgnoreCase)) throw new InvalidDataException("Data verification failed.");
                    if (File.Exists(target)) { result.backup = target + ".previous-" + Guid.NewGuid().ToString("N"); File.Replace(temporary, target, result.backup); }
                    else File.Move(temporary, target);
                    result.changed = true; return result;
                } finally { if (File.Exists(temporary)) File.Delete(temporary); }
            }
            internal void Commit() { if (backup != null && File.Exists(backup)) { try { File.Delete(backup); } catch (Exception e) { App.Log("Data backup cleanup: " + e); } } }
            internal void Rollback() {
                if (!changed) return;
                if (backup == null) { if (File.Exists(target)) File.Delete(target); }
                else if (File.Exists(target)) File.Replace(backup, target, null); else File.Move(backup, target);
            }
        }
        internal static void RemoveStaging(string path, string parent) {
            string full = Path.GetFullPath(path).TrimEnd('\\'), prefix = Path.GetFullPath(parent).TrimEnd('\\') + "\\";
            if (!full.StartsWith(prefix, StringComparison.OrdinalIgnoreCase) || !(Path.GetFileName(full).StartsWith(".staging-", StringComparison.Ordinal) || Path.GetFileName(full).StartsWith("ASRGame-Setup-", StringComparison.Ordinal))) throw new IOException("Invalid staging path.");
            if (Directory.Exists(full) && (File.GetAttributes(full) & FileAttributes.ReparsePoint) == 0) Directory.Delete(full, true);
        }
    }
}
