using System.Diagnostics;

namespace Regis;

using System.IO;

/// <summary>
/// A collection of functionality used to send and receive information over the network.
/// </summary>
public static class Network {
    /// <summary>
    /// Represents the default buffer size used for sending.
    /// </summary>
    private const int DefaultBufferSize = 4096;

    /// <summary>
    /// Sends a byte buffer over the network. This data will be sent with a pre-flight of the buffer length.
    /// Note that exceptions may vary based on deriving classes of <seealso cref="Stream"/>. Please refer to the classes documentation for <seealso cref="Stream"/>.
    /// Data will be sent in <see href="packetSize"/> chunks.
    /// If the data is not a perfect multiple of <see href="packetSize"/>, then the last chuck may have less than <see href="packetSize"/> bytes.
    /// </summary>
    /// <param name="source">The data to send over the stream.</param>
    /// <param name="over">Any stream to send the data over.</param>
    /// <param name="packetSize">The number of bytes to try and send in waves.</param>
    /// <param name="cancellationToken">A token passed to cancel the operation.</param>
    /// <exception cref="OperationCanceledException">If the operation is cancelled.</exception>
    public static async Task SendBuffer(byte[] source, Stream over, int packetSize = DefaultBufferSize, CancellationToken cancellationToken = default) {
        int dataLength = source.Length;
        byte[] encodedLength = BitConverter.GetBytes(dataLength);
        if (BitConverter.IsLittleEndian)
            Array.Reverse(encodedLength);
        
        await over.WriteAsync(encodedLength.AsMemory(0, sizeof(int)), cancellationToken).ConfigureAwait(false);

        // This keeps track of how many bytes have been written so far.
        int totalWritten = 0;
        while (totalWritten < dataLength) {
            // Since the data can have at most packetSize sent at one time, we must check.
            // This will determine if the packet size is larger, or if we should just send what is left. 
            int remaining = dataLength - totalWritten;
            int toWrite = Math.Min(remaining, packetSize);
            
            await over.WriteAsync(source.AsMemory(totalWritten, toWrite), cancellationToken).ConfigureAwait(false);
            totalWritten += toWrite;
        }
    }

    /// <summary>
    /// Attempts to collect a message sent over the network.
    /// The result value will be the completed buffer, except the preflight length.
    /// </summary>
    /// <param name="over">The stream to collect information out of</param>
    /// <param name="packetSize">The number of bytes to try and collect in each wave.</param>
    /// <param name="cancellationToken">A token passed to cancel the operation.</param>
    /// <returns>The bytes sent over the network.</returns>
    /// <exception cref="EndOfStreamException">Occurs if the stream cannot read the data that is expected. This can occur when the header is being read. If the length is not 4 bytes, then this will occur.</exception>
    /// <exception cref="OperationCanceledException">Occurs if the operation is cancelled.</exception>
    public static async ValueTask<List<byte>> ReceiveBuffer(Stream over, int packetSize = DefaultBufferSize, CancellationToken cancellationToken = default) {
        byte[] lengthBuffer = new byte[sizeof(int)];
        await over.ReadExactlyAsync(lengthBuffer.AsMemory(0, sizeof(int)), cancellationToken).ConfigureAwait(false);

        if (BitConverter.IsLittleEndian)
            Array.Reverse(lengthBuffer);

        int dataLength = BitConverter.ToInt32(lengthBuffer);

        List<byte> result = [];
        byte[] buffer = new byte[packetSize];
        while (true) {
            int read = await over.ReadAsync(buffer.AsMemory(0, packetSize), cancellationToken).ConfigureAwait(false);
            result.AddRange(buffer);
            
            if (read == 0 || result.Count == dataLength)
                break;
        }

        return result;
    }
}