using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Security.Cryptography;

namespace Shear
{
    public class PartialFingerprint
    {
        public long Length;

        public PartialFingerprint(FileInfo file)
        {
            Length = file.Length;
        }

        public override bool Equals(object obj)
        {
            var other = obj as PartialFingerprint;
            if (other == null)
                return false;

            return Length == other.Length;
        }

        public override int GetHashCode()
        {
            return Length.GetHashCode();
        }
    }

    public class Fingerprint
    {
        private const int MaxSize = 0x800000;

        public long Length;

        byte[] _head;
        byte[] _tail;

        public IEnumerable<byte> Head => _head;
        public IEnumerable<byte> Tail => _tail == null
            ? Enumerable.Empty<byte>()
            : _tail;

        public Fingerprint(string path)
        {
            var file = File.OpenRead(path);

            Length = file.Length;
            _head = Hash(file, file.Length > MaxSize ? MaxSize : (int)file.Length);
            _tail = null as byte[];

            if (Length > MaxSize)
            {
                var tailLength = MaxSize > Length
                    ? Math.Min(MaxSize, (int)Length - MaxSize)
                    : MaxSize;

                file.Seek(tailLength, SeekOrigin.End);
                _tail = Hash(file, tailLength);
            }
        }

        public override bool Equals(object obj)
        {
            var other = obj as Fingerprint;
            if (other == null)
                return false;

            return Length == other.Length
                && Head.SequenceEqual(other.Head)
                && Tail.SequenceEqual(other.Tail);
        }

        public override int GetHashCode()
        {
            var hash = Length.GetHashCode();
            hash = hash * 17 + HashArray(_head);

            if (_tail != null)
            {
                hash = hash * 17 + HashArray(_tail);
            }

            return hash;
        }

        private static byte[] Hash(Stream stream, int size)
        {
            var hasher = SHA256.Create();
            var buffer = new byte[size];

            var bytesRead = stream.Read(buffer, 0, size);

            hasher.ComputeHash(buffer, 0, bytesRead);
            return hasher.Hash;
        }

        private static int HashArray(byte[] array)
        {
            int hash = 17;
            foreach (var element in array)
            {
                hash = hash * 31 + element;
            }
            return hash;
        }
    }
}
