import sqlite3

# Define the connection to the source SQLite database
source_conn = sqlite3.connect('facturen.db.old')
source_cursor = source_conn.cursor()

# Define the connection to the destination SQLite database
destination_conn = sqlite3.connect('facturen.db')
destination_cursor = destination_conn.cursor()

try:
    # Fetch data from the source table
    source_cursor.execute("SELECT * FROM client")
    rows = source_cursor.fetchall()
    
    # Insert data into the destination table
    destination_cursor.executemany("INSERT INTO client (id, name, address, zip) VALUES (?, ?, ?, ?)", rows)
    destination_conn.commit()
    
    print(f"Successfully migrated {len(rows)} rows.")
except sqlite3.Error as e:
    print(f"An error occurred: {e}")
finally:
    # Close the database connections
    source_conn.close()
    destination_conn.close()

