# telegram-image-search
Telegram Image Search is a CLI application for searching images by text.
How it works:
 - The application creates a dedicated Telegram channel.
 - It runs in the background, waiting for incoming messages.
 - When an image is received, it extracts the text from it.
 - The image is then forwarded to the channel along with the extracted text.
 - Users can find images by searching for text in the channel.

**Instalation**

To use telegram-image-search tesseract and tesseract-data-eng packages are needed.

To compile and run app simply use ``cargo run``
