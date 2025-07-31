using System.Text;
using System.Text.Json;

namespace Regis;

public enum HttpCode {
    Continue                      = 100,
    SwitchingProtocols            = 101,
    Processing                    = 102,
    EarlyHints                    = 103,
    Ok                            = 200,
    Created                       = 201,
    Accepted                      = 202,
    NonAuthoritativeInformation   = 203,
    NoContent                     = 204,
    ResetContent                  = 205,
    PartialContent                = 206,
    MultiStatus                   = 207,
    AlreadyReported               = 208,
    ImUsed                        = 226,
    MultipleChoices               = 300,
    MovedPermanently              = 301,
    Found                         = 302,
    SeeOther                      = 303,
    NotModified                   = 304,
    UseProxy                      = 305,
    TemporaryRedirect             = 307,
    PermanentRedirect             = 308,
    BadRequest                    = 400,
    Unauthorized                  = 401,
    PaymentRequired               = 402,
    Forbidden                     = 403,
    NotFound                      = 404,
    MethodNotAllowed              = 405,
    NotAcceptable                 = 406,
    ProxyAuthenticationRequired   = 407,
    RequestTimeout                = 408,
    Conflict                      = 409,
    Gone                          = 410,
    LengthRequired                = 411,
    PreconditionFailed            = 412,
    PayloadTooLarge               = 413,
    UriTooLong                    = 414,
    UnsupportedMediaType          = 415,
    RangeNotSatisfiable           = 416,
    ExpectationFailed             = 417,
    ImATeapot                     = 418,
    MisdirectedRequest            = 421,
    UnprocessableEntity           = 422,
    Locked                        = 423,
    FailedDependency              = 424,
    TooEarly                      = 425,
    UpgradeRequired               = 426,
    PreconditionRequired          = 428,
    TooManyRequests               = 429,
    RequestHeaderFieldsTooLarge   = 431,
    UnavailableForLegalReasons    = 451,
    InternalServerError           = 500,
    NotImplemented                = 501,
    BadGateway                    = 502,
    ServiceUnavailable            = 503,
    GatewayTimeout                = 504,
    HttpVersionNotSupported       = 505,
    VariantAlsoNegotiates         = 506,
    InsufficientStorage           = 507,
    LoopDetected                  = 508,
    NotExtended                   = 510,
    NetworkAuthenticationRequired = 511,
}

/// <summary>
/// A collection of functions that can be used to send and recieve classes as JSON encoded text.
/// </summary>
public static class MessageManager {
    /// <summary>
    /// Decodes a type <typeparamref name="T"/> from a stream. 
    /// </summary>
    /// <typeparam name="T">The target result type</typeparam>
    /// <param name="data">The data to send over the stream</param>
    /// <param name="over">The stream to send data over</param>
    /// <param name="cancellationToken">A token to cancel the async action.</param>
    /// <exception cref="NotSupportedException">Occurs when <typeparamref name="T"/> cannot be seralized to JSON.</exception>
    /// <exception cref="EncoderFallbackException">Occurs if the data being encoded cannot be turned into <code>byte[]</code>. This should not occur, but is here just in case.</exception>
    /// <exception cref="OperationCanceledException">If the operation was cancelled.</exception>
    /// <returns></returns>
    public static async Task SendMessage<T>(T data, Stream over, CancellationToken cancellationToken = default) where T: notnull {
        string message = JsonSerializer.Serialize<T>(data);
        byte[] bytes = Encoding.UTF8.GetBytes(message);
        await Network.SendBuffer(bytes, over, cancellationToken: cancellationToken).ConfigureAwait(false);
    }

    /// <summary>
    /// Recieves a JSON encoded type from a stream. 
    /// </summary>
    /// <typeparam name="T">The type to deserialize to.</typeparam>
    /// <param name="over">The stream to extract from</param>
    /// <param name="cancellationToken">A token to cancel the operation.</param>
    /// <returns>The deserialized data.</returns>
    /// <exception cref="EndOfStreamException">If the inner stream could not read data properly.</exception>
    /// <exception cref="JsonException">If the data could not be deserialized into <see href="T"/>, or if the deserialization resulted in a null value.</exception>
    /// /// <exception cref="OperationCanceledException">If the operation was cancelled.</exception>
    public static async ValueTask<T> RecvMessage<T>(Stream over, CancellationToken cancellationToken = default) where T: notnull {
        List<byte> buffer = await Network.ReceiveBuffer(over, cancellationToken: cancellationToken).ConfigureAwait(false);
        string message = Encoding.UTF8.GetString([.. buffer]);
        T? result = JsonSerializer.Deserialize<T>(message);
        return result is null
            ? throw new JsonException("the content of the JSON document resulted in a null T, and this is dissallowed.")
            : result;
    }
}
