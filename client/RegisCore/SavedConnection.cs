using System.Net;
using System.Configuration;

namespace Regis {
    public class SavedConnection {
        public required IPAddress Address { get; set; }
        public required string Name { get; set; }
    }

    public class ConnectionManager {
        private string configPath;
    }
}
