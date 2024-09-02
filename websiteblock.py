import os
import platform

# Function to modify hosts file
def modify_hosts_file(blocked_sites, unblock=False):
    hosts_path = {
        "Windows": r"C:\Windows\System32\drivers\etc\hosts",
        "Darwin": "/etc/hosts"
    }[platform.system()]

    redirect_ip = "127.0.0.1"
    
    with open(hosts_path, 'r+') as file:
        lines = file.readlines()
        file.seek(0)
        for line in lines:
            if not any(site in line for site in blocked_sites):
                file.write(line)
        if not unblock:
            for site in blocked_sites:
                file.write(f"{redirect_ip} {site}\n")
        file.truncate()

    # Flush DNS cache
    if platform.system() == "Windows":
        os.system("ipconfig /flushdns")
    else:
        os.system("sudo dscacheutil -flushcache; sudo killall -HUP mDNSResponder")

# Example usage
#blocked_sites = ["www.example.com", "www.blockme.com"]
#modify_hosts_file(blocked_sites)

import hashlib

def hash_password(password):
    return hashlib.sha256(password.encode()).hexdigest()

def check_password(input_password, stored_hash):
    return hash_password(input_password) == stored_hash

# Store the password securely after first run
stored_password_hash = hash_password("1234")

# On unblock or uninstall
input_password = input("Enter your password: ")
if check_password(input_password, stored_password_hash):
    print("Password correct! Proceeding...")
else:
    print("Incorrect password!")
    quit()


from PyQt5.QtWidgets import QApplication, QWidget, QVBoxLayout, QPushButton, QLineEdit, QListWidget

class BlockerApp(QWidget):
    def __init__(self):
        super().__init__()
        self.initUI()
    
    def initUI(self):
        layout = QVBoxLayout()
        
        self.website_input = QLineEdit(self)
        self.website_input.setPlaceholderText("Enter website to block")
        layout.addWidget(self.website_input)
        
        self.add_button = QPushButton("Add to Blocklist", self)
        self.add_button.clicked.connect(self.add_website)
        layout.addWidget(self.add_button)
        
        self.blocked_list = QListWidget(self)
        layout.addWidget(self.blocked_list)
        
        #self.remove_button = QPushButton("Remove Selected", self)
        #self.remove_button.clicked.connect(self.remove_website)
        #layout.addWidget(self.remove_button)
        
        self.setLayout(layout)
        self.setWindowTitle('Website Blocker')
        self.show()
    
    def add_website(self):
        site = self.website_input.text()
        if site:
            self.blocked_list.addItem(site)
            modify_hosts_file([site])
            self.website_input.clear()
    
    def remove_website(self):
        selected_item = self.blocked_list.currentItem()
        if selected_item:
            site = selected_item.text()
            modify_hosts_file([site], unblock=True)
            self.blocked_list.takeItem(self.blocked_list.row(selected_item))

app = QApplication([])
ex = BlockerApp()
app.exec_()
