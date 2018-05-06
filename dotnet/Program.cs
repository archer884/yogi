using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;

namespace Shear
{
    class Program
    {
        static void Main(string[] args)
        {
            if (args.Length != 1)
            {
                Console.Error.WriteLine("Please provide a root directory");
                Environment.Exit(1);
            }

            var root = args[0];

            var filesByPartialFingerprint = new Dictionary<PartialFingerprint, List<string>>();
            foreach (var file in WalkFromRoot(root))
            {
                var partial = new PartialFingerprint(file);
                if (filesByPartialFingerprint.ContainsKey(partial))
                {
                    filesByPartialFingerprint[partial].Add(file.FullName);
                }
                else
                {
                    filesByPartialFingerprint.Add(partial, new List<string> { file.FullName });
                }
            }

            var filesByFingerprint = new Dictionary<Fingerprint, List<string>>();
            foreach (var paths in filesByPartialFingerprint.Values)
            {
                if (paths.Count < 2)
                {
                    continue;
                }

                foreach (var path in paths)
                {
                    var fingerprint = new Fingerprint(path);
                    if (filesByFingerprint.ContainsKey(fingerprint))
                    {
                        filesByFingerprint[fingerprint].Add(path);
                    }
                    else
                    {
                        filesByFingerprint.Add(fingerprint, new List<string> { path });
                    }
                }
            }

            foreach (var paths in filesByFingerprint.Values.Where(paths => paths.Count > 1))
            {
                foreach (var path in paths.OrderByDescending(path => path.Length).Skip(1))
                {
                    Console.WriteLine(path);
                }
            }
        }

        static IEnumerable<FileInfo> WalkFromRoot(string root)
        {
            if (!Directory.Exists(root))
            {
                return Enumerable.Empty<FileInfo>();
            }

            return WalkFromDirectory(new DirectoryInfo(root));
        }

        // Try writing this in Rust...
        static IEnumerable<FileInfo> WalkFromDirectory(DirectoryInfo directory)
        {
            foreach (var file in directory.GetFiles())
            {
                yield return file;
            }

            foreach (var subDirectory in directory.GetDirectories())
            {
                foreach (var file in WalkFromDirectory(subDirectory))
                {
                    yield return file;
                }
            }
        }
    }
}
