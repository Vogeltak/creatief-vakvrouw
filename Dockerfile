FROM rust:1.68.0-bullseye

WORKDIR /usr/src/app
COPY . .

RUN apt-get update && apt-get install -y fonts-liberation2 texlive-latex-recommended texlive-latex-extra texlive-xetex pandoc

RUN cargo install --path .

CMD ["creatief-vakvrouw", "--month", "2023-03"]
#CMD ["pandoc", "/usr/src/app/templates/invoice/details.yml", "-o", "invoice.pdf", "--template=/usr/src/app/templates/invoice/template.tex", "--pdf-engine=xelatex"]

