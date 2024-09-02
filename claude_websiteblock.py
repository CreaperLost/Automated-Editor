import tkinter as tk
from tkinter import simpledialog, messagebox
import hashlib
import os
import subprocess

# macOS hosts file path
HOSTS_PATH = "/etc/hosts"
REDIRECT = "127.0.0.1"

# In a real application, you'd want to store this securely
# This is just a simple example
PASSWORD_HASH = hashlib.sha256("1234".encode()).hexdigest()

def verify_password():
    password = simpledialog.askstring("Password", "Enter password:", show='*')
    if password:
        return hashlib.sha256(password.encode()).hexdigest() == PASSWORD_HASH
    return False

def run_as_root(command, input_data=None):
    try:
        return subprocess.run(['sudo', '-S'] + command, 
                              input=input_data.encode() if input_data else b'your_sudo_password\n',  # Replace with actual sudo password
                              capture_output=True, text=True, check=True)
    except subprocess.CalledProcessError as e:
        print(f"Error running command: {e}")
        return None

def block_websites(websites):
    lines_to_add = [f"{REDIRECT} {website}\n" for website in websites]
    command = ['tee', '-a', HOSTS_PATH]
    result = run_as_root(command, input=''.join(lines_to_add))
    if result and result.returncode == 0:
        messagebox.showinfo("Success", "Websites blocked successfully")
    else:
        messagebox.showerror("Error", "Failed to block websites")

def unblock_websites(websites):
    if not verify_password():
        messagebox.showerror("Error", "Incorrect password")
        return
    
    # Read current content
    with open(HOSTS_PATH, 'r') as file:
        lines = file.readlines()
    
    # Filter out lines containing blocked websites
    new_lines = [line for line in lines if not any(website in line for website in websites)]
    
    # Write back to file
    command = ['tee', HOSTS_PATH]
    result = run_as_root(command, input=''.join(new_lines))
    if result and result.returncode == 0:
        messagebox.showinfo("Success", "Websites unblocked successfully")
    else:
        messagebox.showerror("Error", "Failed to unblock websites")

def add_websites():
    websites = simpledialog.askstring("Add Websites", "Enter comma-separated websites:")
    if websites:
        websites = [w.strip() for w in websites.split(',')]
        block_websites(websites)
        update_listbox()

def remove_websites():
    selection = listbox.curselection()
    if selection:
        websites = [listbox.get(index) for index in selection]
        unblock_websites(websites)
        update_listbox()

def update_listbox():
    listbox.delete(0, tk.END)
    with open(HOSTS_PATH, 'r') as file:
        for line in file:
            if line.startswith(REDIRECT):
                listbox.insert(tk.END, line.split()[1])

# Main window
root = tk.Tk()
root.title("Website Blocker for macOS")
root.geometry("300x400")

# Listbox for showing blocked websites
listbox = tk.Listbox(root, selectmode=tk.MULTIPLE)
listbox.pack(pady=10, padx=10, expand=True, fill=tk.BOTH)

# Buttons
tk.Button(root, text="Add Websites", command=add_websites).pack(pady=5)
tk.Button(root, text="Remove Selected", command=remove_websites).pack(pady=5)

# Initial update of listbox
update_listbox()

# Overriding the close button
def on_closing():
    if verify_password():
        root.destroy()
    else:
        messagebox.showerror("Error", "Incorrect password")

root.protocol("WM_DELETE_WINDOW", on_closing)

root.mainloop()