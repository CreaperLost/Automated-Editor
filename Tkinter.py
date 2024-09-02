import tkinter as tk
from tkinter import filedialog, messagebox
from tkVideoPlayer import TkinterVideo
from UI.button_usage import *
from video_editor.process_pipe import process_videos

# Fonts
title_font = ("Helvetica", 25, "bold")
description_font = ("Helvetica", 15, "bold")
button_font = ("Helvetica", 18, "bold")

class AeroEditorApp(tk.Tk):
    def __init__(self):
        super().__init__()
        self.title("AeroEditor")
        self.geometry("800x800")
        self.resizable(True, True)
        self._show_main_menu()

    def _show_main_menu(self):
        # Clear the window
        for widget in self.winfo_children():
            widget.destroy()

        title_label = tk.Label(self, text="AeroEditor", font=title_font)# Create a label for the title and description
        title_label.pack(pady=20)

        description_label = tk.Label(self, text="Automated Video Editor", font=description_font)
        description_label.pack()

        # Frame to contain the buttons
        button_frame = tk.Frame(self)
        button_frame.pack(fill="both", expand=True, padx=20, pady=10)

        # Configure the grid layout for the buttons
        button_frame.grid_columnconfigure(0, weight=1)  # Make the single column expandable
        button_frame.grid_rowconfigure(0, weight=1)
        button_frame.grid_rowconfigure(1, weight=1)
        button_frame.grid_rowconfigure(2, weight=1)
        button_frame.grid_rowconfigure(3, weight=1)
        button_frame.grid_rowconfigure(4, weight=1)
        button_frame.grid_rowconfigure(5, weight=1)
        button_frame.grid_rowconfigure(6, weight=1)

        # Create buttons for the different functions
        sync_clips_button = tk.Button(button_frame, text="Sync Clips", font=button_font, padx=20, pady=10,command=self._open_sync_page)
        sync_clips_button.grid(row=0, column=0, sticky="nsew", pady=5)

        remove_silences_button = tk.Button(button_frame, text="Remove Silences", font=button_font, padx=20, pady=10, command=remove_silences)
        remove_silences_button.grid(row=1, column=0, sticky="nsew", pady=5)

        merge_videos_button = tk.Button(button_frame, text="Merge Videos", font=button_font, padx=20, pady=10, command=merge_videos)
        merge_videos_button.grid(row=2, column=0, sticky="nsew", pady=5)

        transcribe_audio_button = tk.Button(button_frame, text="Transcribe Audio", font=button_font, padx=20, pady=10, command=transcribe_audio)
        transcribe_audio_button.grid(row=2, column=0, sticky="nsew", pady=5)

        add_captions_button = tk.Button(button_frame, text="Add Captions", font=button_font, padx=20, pady=10, command=add_captions)
        add_captions_button.grid(row=3, column=0, sticky="nsew", pady=5)

        add_music_button = tk.Button(button_frame, text="Add Music", font=button_font, padx=20, pady=10, command = add_music)
        add_music_button.grid(row=4, column=0, sticky="nsew", pady=5)

        custom_pipeline_button = tk.Button(button_frame, text="Custom Pipeline", font=button_font, padx=20, pady=10, command=custom_pipeline)
        custom_pipeline_button.grid(row=5, column=0, sticky="nsew", pady=5)

    def _open_sync_page(self):
        # Clear the window
        for widget in self.winfo_children():
            widget.destroy()

        # Variables to hold the file paths
        self.left_video_path = None
        self.right_video_path = None

        # Back button
        back_button = tk.Button(self, text="Back", command=self._show_main_menu)
        back_button.pack(anchor="nw", padx=10, pady=10)

        # Create two frames for the split view
        split_frame = tk.Frame(self)
        split_frame.pack(fill="both", expand=True, padx=10, pady=10)

        left_frame = tk.Frame(split_frame, bg="lightgray", width=400, height=300)
        left_frame.pack(side="left", fill="both", expand=True, padx=5, pady=5)

        right_frame = tk.Frame(split_frame, bg="lightgray", width=400, height=300)
        right_frame.pack(side="right", fill="both", expand=True, padx=5, pady=5)

        self.left_label = tk.Label(left_frame, text="Drag-Drop or Click Here", bg="lightgray")
        self.left_label.pack(expand=True)

        self.right_label = tk.Label(right_frame, text="Drag-Drop or Click Here", bg="lightgray")
        self.right_label.pack(expand=True)

        # Bind drag-and-drop or allow file explorer
        left_frame.bind("<Button-1>", lambda event: self._select_video("left", left_frame))
        right_frame.bind("<Button-1>", lambda event: self._select_video("right", right_frame))

        # SYNC button at the bottom
        sync_button = tk.Button(self, text="SYNC", state="disabled", command=self._sync_clips)
        sync_button.pack(side="bottom", pady=10)

        self.sync_button = sync_button
        self.left_frame = left_frame
        self.right_frame = right_frame

    def _select_video(self, side, frame):
        if side == "left":
            if not self.left_video_path:
                self.left_video_path = filedialog.askopenfilename(filetypes=[("Video Files", "*.mp4 *.mov *.avi")])
                if self.left_video_path:
                    self.left_label.destroy()  # Remove the instruction label
                    self._preview_video(side, frame)
        elif side == "right":
            if not self.right_video_path:
                self.right_video_path = filedialog.askopenfilename(filetypes=[("Video Files", "*.mp4 *.mov *.avi")])
                if self.right_video_path:
                    self.right_label.destroy()
                    self._preview_video(side, frame)

        # Enable SYNC button if both videos are selected
        if self.left_video_path and self.right_video_path:
            self.sync_button.config(state="normal")

    def _preview_video(self, side, frame):
        video_path = self.left_video_path if side == "left" else self.right_video_path
        video_player = TkinterVideo(master=frame, scaled=True)
        video_player.load(video_path)
        video_player.pack(expand=True, fill="both")
        video_player.set_size((frame.winfo_width(), frame.winfo_height()))

        # Create a play button
        play_button = tk.Button(frame, text="Play", command=video_player.play)
        play_button.place(relx=0.5, rely=0.5, anchor="center")

        # Create a delete button
        delete_button = tk.Button(frame, text="Delete Clip", command=lambda: self._delete_video(side, video_player, play_button, delete_button))
        delete_button.place(relx=0.95, rely=0.05, anchor="ne")

        # Disable frame interaction
        frame.unbind("<Button-1>")

    def _delete_video(self, side, video_player, play_button, delete_button):
        video_player.destroy()
        play_button.destroy()
        delete_button.destroy()

        if side == "left":
            self.left_video_path = None
            self.left_label = tk.Label(self.left_frame, text="Drag-Drop or Click Here", bg="lightgray")
            self.left_label.pack(expand=True)
            self.left_frame.bind("<Button-1>", lambda event: self._select_video("left", self.left_frame))
        elif side == "right":
            self.right_video_path = None
            self.right_label = tk.Label(self.right_frame, text="Drag-Drop or Click Here", bg="lightgray")
            self.right_label.pack(expand=True)
            self.right_frame.bind("<Button-1>", lambda event: self._select_video("right", self.right_frame))
        
        # Disable SYNC button if both video are deleted
        if self.left_video_path or self.right_video_path:
            self.sync_button.config(state="disabled")

    def _sync_clips(self):
        
        if self.left_video_path and self.right_video_path:
            # Example functionality for syncing clips
            messagebox.showinfo("Sync", "Syncing the selected clips...")
            process_videos( video_1= { "path": self.left_video_path, "track":0},
                            video_2=  {"path": self.right_video_path, "track":0})
            # Example functionality for syncing clips
            #messagebox.showinfo("Finished", "Finalized, Open File Location")
            return
        # Example functionality for syncing clips
        messagebox.showinfo("Error", "Select 2 video clips.")

