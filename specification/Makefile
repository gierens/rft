all: draft-rft.txt

%.txt: %.xml
	xml2rfc $<

%.xml: %.md
	kramdown-rfc $< > $@

clean:
	rm -f *.xml *.txt
