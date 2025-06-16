using System.ComponentModel;
using System.Text;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Documents;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Navigation;
using System.Windows.Shapes;

namespace Regis.Gui {
    /// <summary>
    /// Interaction logic for MainWindow.xaml
    /// </summary>
    public partial class MainWindow : Window, INotifyPropertyChanged {
        public MainWindow() {
            InitializeComponent();
            DataContext = this;
        }

        private string _FooterText = "Ready";
        public string FooterText {
            get => _FooterText;
            set {
                if (_FooterText != value) {
                    _FooterText = value;
                    OnPropertyChanged(nameof(FooterText));
                }
            }
        }

        public event PropertyChangedEventHandler? PropertyChanged;
        public void OnPropertyChanged(string name) => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));

        private void OpenConnection_Click(object sender, RoutedEventArgs e) {

        }

        private void CloseConnection_Click(object sender, RoutedEventArgs e) {

        }

        private void Refresh_Click(object sender, RoutedEventArgs e) {

        }

        private void PauseCollection_Click(object sender, RoutedEventArgs e) {

        }

        private void ResumeCollection_Click(object sender, RoutedEventArgs e) {

        }

        private void ConnDetails_Click(object sender, RoutedEventArgs e) {

        }

        private void NewWindow_Click(object sender, RoutedEventArgs e) {

        }

        private void Homepage_Click(object sender, RoutedEventArgs e) {

        }

        private void About_Click(object sender, RoutedEventArgs e) {

        }

        private void Help_Click(object sender, RoutedEventArgs e) {

        }

        private void BugReporter_Click(object sender, RoutedEventArgs e) {

        }

        private void Settings_Click(object sender, RoutedEventArgs e) {

        }

        private void Quit_Click(object sender, RoutedEventArgs e) {

        }

        private void CloseCurrEditor_Click(object sender, RoutedEventArgs e) {

        }

        private void PopoutCurrentEditor_Click(object sender, RoutedEventArgs e) {

        }

        private void CloseAllEditor_Click(object sender, RoutedEventArgs e) {

        }
    }
}