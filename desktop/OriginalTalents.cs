// Read-only migration from the player's original RSE1 save. No game keys or saves are bundled.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Text.RegularExpressions;
using System.Windows.Forms;
using Microsoft.Win32;

namespace ASRDesktop {
    static class OriginalTalents {
        sealed class Section { internal int rva, offset, length; }

        static byte[] Mac(byte[] key, params byte[][] parts) {
            using (var input = new MemoryStream()) {
                foreach (var part in parts) input.Write(part, 0, part.Length);
                using (var hmac = new HMACSHA1(key)) return hmac.ComputeHash(input.ToArray());
            }
        }
        static string Fingerprint(byte[] bytes) {
            using (var sha = SHA256.Create()) return BitConverter.ToString(sha.ComputeHash(bytes));
        }
        static bool Fits(byte[] bytes, int offset, int size) { return offset >= 0 && size >= 0 && offset <= bytes.Length - size; }

        // Locate paired SHA-1 HMAC ipad/opad loads in the player's x64 PE executable.
        static IEnumerable<byte[]> Keys(string path) {
            if (new FileInfo(path).Length > 512L * 1024 * 1024) yield break;
            byte[] data = File.ReadAllBytes(path);
            if (!Fits(data, 0, 64) || data[0] != 'M' || data[1] != 'Z') yield break;
            int pe = BitConverter.ToInt32(data, 60);
            if (!Fits(data, pe, 24) || BitConverter.ToUInt32(data, pe) != 0x4550) yield break;
            int count = BitConverter.ToUInt16(data, pe + 6), optional = BitConverter.ToUInt16(data, pe + 20);
            if (!Fits(data, pe + 24, optional) || optional < 2 || BitConverter.ToUInt16(data, pe + 24) != 0x20b) yield break;
            var sections = new List<Section>(); int lo = 0, hi = data.Length;
            for (int n = 0; n < count; n++) {
                int at = pe + 24 + optional + n * 40;
                if (!Fits(data, at, 40)) yield break;
                int offset = BitConverter.ToInt32(data, at + 20), length = BitConverter.ToInt32(data, at + 16);
                if (!Fits(data, offset, length)) yield break;
                sections.Add(new Section { rva = BitConverter.ToInt32(data, at + 12), offset = offset, length = length });
                if (Encoding.ASCII.GetString(data, at, 8).TrimEnd('\0') == ".text") { lo = offset; hi = offset + length; }
            }
            byte[] previous = null; int previousAt = 0;
            for (int i = lo + 7; i < hi - 4; i++) {
                if (data[i] != 0xf0 || data[i + 1] != 0xe1 || data[i + 2] != 0xd2 || data[i + 3] != 0xc3
                    || data[i - 7] != 0xc7 || data[i - 6] != 0x84 || data[i - 5] != 0x24) continue;
                var block = new List<byte>();
                for (int j = i + 4; j + 7 < Math.Min(i + 133, hi) && block.Count < 64; j++) {
                    if (data[j] != 0x0f || (data[j + 1] != 0x28 && data[j + 1] != 0x10) || (data[j + 2] & 0xc7) != 5) continue;
                    var instruction = sections.FirstOrDefault(s => j + 7 >= s.offset && j + 7 < s.offset + s.length);
                    if (instruction == null) continue;
                    long target = (long)instruction.rva + j + 7 - instruction.offset + BitConverter.ToInt32(data, j + 3);
                    var source = sections.FirstOrDefault(s => target >= s.rva && target - s.rva <= s.length - 16);
                    if (source != null) {
                        byte[] chunk = new byte[16];
                        Buffer.BlockCopy(data, source.offset + (int)(target - source.rva), chunk, 0, 16);
                        block.AddRange(chunk);
                    }
                    j += 6;
                }
                if (block.Count != 64) continue;
                byte[] next = block.ToArray();
                if (previous != null && i - previousAt <= 4096) {
                    byte[] key = previous.Select(b => (byte)(b ^ 0x36)).ToArray();
                    if (key.SequenceEqual(next.Select(b => (byte)(b ^ 0x5c)))) yield return key;
                }
                previous = next; previousAt = i;
            }
        }

        static string Open(byte[] sealedSave, byte[] key) {
            if (sealedSave.Length < 36) throw new InvalidDataException("The original save is incomplete.");
            byte[] nonce = sealedSave.Take(16).ToArray();
            byte[] cipher = sealedSave.Skip(16).Take(sealedSave.Length - 36).ToArray();
            byte[] tag = Mac(key, Encoding.ASCII.GetBytes("tag"), nonce, cipher);
            int difference = 0;
            for (int i = 0; i < 20; i++) difference |= tag[i] ^ sealedSave[sealedSave.Length - 20 + i];
            if (difference != 0) return null;
            for (int at = 0; at < cipher.Length; at += 20) {
                byte[] counter = BitConverter.GetBytes((ulong)(at / 20));
                if (!BitConverter.IsLittleEndian) Array.Reverse(counter);
                byte[] stream = Mac(key, Encoding.ASCII.GetBytes("ks"), nonce, counter);
                for (int i = 0; i < Math.Min(20, cipher.Length - at); i++) cipher[at + i] ^= stream[i];
            }
            return new UTF8Encoding(false, true).GetString(cipher);
        }

        static IEnumerable<string> Executables() {
            string custom = Environment.GetEnvironmentVariable("ASR_ORIGINAL_EXE");
            if (!String.IsNullOrWhiteSpace(custom)) yield return custom;
            var folders = new List<string>();
            using (var key = Registry.CurrentUser.OpenSubKey(@"Software\Microsoft\Windows\CurrentVersion\Uninstall\ASR")) {
                string folder = key == null ? null : key.GetValue("InstallLocation") as string;
                if (!String.IsNullOrWhiteSpace(folder)) folders.Add(folder);
            }
            string local = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
            folders.Add(Path.Combine(local, "WorldOfSurvive", "game"));
            folders.Add(Path.Combine(local, "ASR", "game"));
            folders.Add(Path.Combine(local, "Programs", "ASR"));
            foreach (string folder in folders) foreach (string relative in new[] { "AzerothSurvivors.exe", "game/AzerothSurvivors.exe", "ASR.exe", "WorldOfSurvive.exe" })
                yield return Path.Combine(folder, relative.Replace('/', Path.DirectorySeparatorChar));
        }

        static string Decrypt(byte[] bytes) {
            string encoded = Encoding.ASCII.GetString(bytes);
            if (!encoded.StartsWith("RSE1:", StringComparison.Ordinal)) throw new InvalidDataException("The original save format is not supported.");
            byte[] sealedSave = Convert.FromBase64String(encoded.Substring(5).Trim());
            foreach (string exe in Executables().Distinct(StringComparer.OrdinalIgnoreCase)) {
                if (!File.Exists(exe)) continue;
                try {
                    foreach (byte[] key in Keys(exe)) {
                        string text = Open(sealedSave, key);
                        if (text != null) return text;
                    }
                } catch (IOException) { } catch (UnauthorizedAccessException) { }
            }
            using (var picker = new OpenFileDialog { Title = "Select the original Azeroth Survivors game to import your talents", Filter = "Original game executable (*.exe)|*.exe", CheckFileExists = true }) {
                if (picker.ShowDialog() != DialogResult.OK) return null;
                foreach (byte[] key in Keys(picker.FileName)) {
                    string text = Open(sealedSave, key);
                    if (text != null) return text;
                }
            }
            throw new InvalidDataException("The selected executable cannot open the original talent save.");
        }

        internal static void Import(string home) {
            // All installed versions share this folder; never race a live game saving its book.
            foreach (var process in Process.GetProcessesByName("warcraft-survivors")) using (process) { return; }
            string source = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "wenilla-rogue", "characters.txt");
            if (!File.Exists(source)) return;
            if (new FileInfo(source).Length > 4 * 1024 * 1024) throw new InvalidDataException("The original save is too large.");
            byte[] bytes = File.ReadAllBytes(source);
            string fingerprint = Fingerprint(bytes), marker = Path.Combine(home, "original-talents-imported-v1.txt");
            if (File.Exists(marker) && File.ReadAllText(marker) == fingerprint) return;
            string plaintext = Decrypt(bytes);
            if (plaintext == null) return;
            string target = Path.Combine(home, "survivors-talents.txt");
            string before = File.Exists(target) ? File.ReadAllText(target) : "";
            var book = App.TalentLines(before); int imported = 0, found = 0;
            foreach (string line in plaintext.Split('\n')) {
                string[] fields = line.Trim().Split('|'); int id; double balance;
                if (fields.Length < 3 || fields[0] != "talent" || !Int32.TryParse(fields[1], out id)
                    || !new[] { 1, 2, 3, 4, 5, 7, 8, 9, 11 }.Contains(id)
                    || !Double.TryParse(fields[2], NumberStyles.Float, CultureInfo.InvariantCulture, out balance)
                    || Double.IsNaN(balance) || Double.IsInfinity(balance) || balance < 0) continue;
                var ranks = new SortedDictionary<string, int>();
                foreach (string token in (fields.Length > 3 ? fields[3] : "").Split(',')) {
                    string[] pair = token.Split(':'); int rank;
                    if (pair.Length == 2 && Regex.IsMatch(pair[0], "^[a-zA-Z0-9_]+$") && Int32.TryParse(pair[1], out rank) && rank > 0 && rank <= 5) ranks[pair[0]] = rank;
                }
                int spent = ranks.Values.Sum(); if (spent > 51) continue;
                found++;
                double xp = balance + 100.0 * spent + 25.0 * spent * (spent - 1) / 2.0;
                string current; bool have = book.TryGetValue(id, out current);
                double existing = have ? App.TalentXp(current) : 0;
                bool hasRanks = have && current.Split((char[])null, StringSplitOptions.RemoveEmptyEntries).Skip(2).Any(w => Regex.IsMatch(w, "=([1-5])$"));
                if (have && existing >= xp && (hasRanks || ranks.Count == 0)) continue;
                string replacement = id + " " + Math.Max(existing, xp).ToString("R", CultureInfo.InvariantCulture) + " "
                    + String.Join(" ", ranks.Select(p => p.Key + "=" + p.Value));
                if (have && current.Trim() == replacement.Trim()) continue;
                book[id] = replacement.Trim(); imported++;
            }
            if (!bytes.SequenceEqual(File.ReadAllBytes(source))) throw new IOException("The original save changed during import. Close the original game and try again.");
            if (File.Exists(target) && File.ReadAllText(target) != before) throw new IOException("The talent book changed during import. Close ASR Game and try again.");
            Directory.CreateDirectory(home);
            if (imported > 0) {
                if (File.Exists(target)) File.Copy(target, target + ".before-original-import-" + Guid.NewGuid().ToString("N"), false);
                string temporary = target + ".new-" + Guid.NewGuid().ToString("N");
                File.WriteAllText(temporary, String.Join("\n", book.OrderBy(p => p.Key).Select(p => p.Value)) + "\n", new UTF8Encoding(false));
                if (File.Exists(target)) File.Replace(temporary, target, null); else File.Move(temporary, target);
            }
            File.WriteAllText(marker, fingerprint, new UTF8Encoding(false));
            App.Log("Original talent migration completed: " + imported + " of " + found + " supported classes imported; higher local progress preserved.");
        }
    }
}
