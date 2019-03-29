# I should really come up with a better way to handle this than what I'm doing.
web: bin/diesel migration run && ROCKET_DATABASES="{more_interesting={url=\"$DATABASE_URL\"}}" ROCKET_SECRET_KEY=`python -c 'import os; import base64; print(base64.b64encode(base64.b16decode(os.getenv("SECRET_KEY"), True)))'` ROCKET_ENV=prod bin/start-nginx ./target/release/more-interesting
