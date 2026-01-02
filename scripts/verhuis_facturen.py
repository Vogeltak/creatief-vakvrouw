import sqlite3

# Define the connection to the source SQLite database
source_conn = sqlite3.connect('facturen.db.old')
source_cursor = source_conn.cursor()

# Define the connection to the destination SQLite database
destination_conn = sqlite3.connect('facturen.db')
destination_cursor = destination_conn.cursor()

try:
    # Fetch data from the source table
    source_cursor.execute("SELECT * FROM invoice")
    rows = source_cursor.fetchall()

    for row in rows:
        # Insert PDF data as a binary blob
        with open(f"{row[1]}.pdf", 'rb') as f:
            pdf_data = f.read()
        destination_cursor.execute("INSERT INTO pdf (file) VALUES (?)", (pdf_data,))
        # Move the factuur row and use lastrowid on the cursor to get the id of the PDF we
        # just inserted and use it for our foreign key
        destination_cursor.execute("INSERT INTO invoice (id, nummer, client, pdf, work_items, subtotal, btw, total, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)", (row[0], row[1], row[2], destination_cursor.lastrowid, row[3], row[4], row[5], row[6], row[7]))
    
    destination_conn.commit()
    
    print(f"Successfully migrated {len(rows)} rows.")
except sqlite3.Error as e:
    print(f"An error occurred: {e}")
finally:
    # Close the database connections
    source_conn.close()
    destination_conn.close()

